use crate::data::{CalendarEntry, CsvBarFetcher, CsvCalendarFetcher, DataBase, DateBar, Fetcher};
use crate::error::{CliError, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser, Debug)]
#[command(version, about = "A quantitative trading tool.")]
pub enum Cli {
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum DbAction {
    Init {
        #[arg(env = "GOOSE_DB")]
        db: PathBuf,
    },
    Import {
        #[arg(long, value_name = "DB", env = "GOOSE_DB")]
        db: PathBuf,
        #[arg(value_name = "TABLE")]
        table: ImportTable,
        #[arg(value_name = "PATH", required = true)]
        paths: Vec<PathBuf>,
        #[arg(short, long, default_value_t = ImportMode::Insert)]
        mode: ImportMode,
        #[arg(short, long, default_value_t = 100, value_parser = parse_batch_size)]
        batch_size: usize,
    },
}

impl DbAction {
    pub fn act(&self) -> Result<()> {
        match self {
            Self::Init { db } => init_database(db),
            Self::Import {
                db,
                table,
                paths,
                mode,
                batch_size,
            } => import_csv(db, table, paths, mode, *batch_size),
        }
    }
}

#[derive(ValueEnum, Debug, Clone)]
pub enum ImportTable {
    Bar,
    Calendar,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum ImportMode {
    Insert,
    Upsert,
}

impl Display for ImportMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ImportMode::*;
        match self {
            Insert => write!(f, "insert"),
            Upsert => write!(f, "upsert"),
        }
    }
}

fn init_database(path: &Path) -> Result<()> {
    let _database = DataBase::new(&path.display().to_string());
    println!("Initialized database at {}.", path.display());
    Ok(())
}

fn import_csv(
    db: &Path,
    table: &ImportTable,
    paths: &[PathBuf],
    mode: &ImportMode,
    batch_size: usize,
) -> Result<()> {
    let mut database = DataBase::new(&db.display().to_string());
    let files = collect_import_files(paths)?;
    let mut affected = 0;

    for file in &files {
        affected += match table {
            ImportTable::Bar => import_bar_file(&mut database, file, mode, batch_size)?,
            ImportTable::Calendar => import_calendar_file(&mut database, file, mode, batch_size)?,
        };
    }

    println!(
        "{} {} rows into {} from {} CSV file(s) for database {}.",
        mode.to_past_tense(),
        affected,
        table,
        files.len(),
        db.display(),
    );
    Ok(())
}

fn import_calendar_file(
    database: &mut DataBase,
    file: &Path,
    mode: &ImportMode,
    batch_size: usize,
) -> Result<usize> {
    let total = count_csv_records(file)?;
    let mut progress = ImportProgress::new(file, total);
    let mut fetcher = CsvCalendarFetcher::from_path(file, batch_size)?;
    let mut affected = 0;

    while let Some(batch) = fetcher.fetch()? {
        progress.inc(batch.len() as u64);
        affected += persist_calendar(database, &batch, mode)?;
    }

    progress.finish();
    Ok(affected)
}

fn import_bar_file(
    database: &mut DataBase,
    file: &Path,
    mode: &ImportMode,
    batch_size: usize,
) -> Result<usize> {
    let total = count_csv_records(file)?;
    let mut progress = ImportProgress::new(file, total);
    let mut fetcher = CsvBarFetcher::from_path(file, batch_size)?;
    let mut affected = 0;

    while let Some(batch) = fetcher.fetch()? {
        progress.inc(batch.len() as u64);
        affected += persist_bars(database, &batch, mode)?;
    }

    progress.finish();
    Ok(affected)
}

fn persist_bars(database: &mut DataBase, bars: &[DateBar], mode: &ImportMode) -> Result<usize> {
    match mode {
        ImportMode::Insert => database.insert_bars(bars),
        ImportMode::Upsert => database.upsert_bars(bars),
    }
}

fn persist_calendar(
    database: &mut DataBase,
    entries: &[CalendarEntry],
    mode: &ImportMode,
) -> Result<usize> {
    match mode {
        ImportMode::Insert => database.insert_calendar(entries),
        ImportMode::Upsert => database.upsert_calendar(entries),
    }
}

fn count_csv_records(file: &Path) -> Result<u64> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(file)?;
    let mut count = 0;
    for record in reader.records() {
        record?;
        count += 1;
    }
    Ok(count)
}

struct ImportProgress<'a> {
    file: &'a Path,
    total: u64,
    current: u64,
}

impl<'a> ImportProgress<'a> {
    fn new(file: &'a Path, total: u64) -> Self {
        let progress = Self {
            file,
            total,
            current: 0,
        };
        progress.draw();
        progress
    }

    fn inc(&mut self, amount: u64) {
        self.current = self.current.saturating_add(amount).min(self.total);
        self.draw();
    }

    fn finish(&self) {
        eprintln!();
    }

    fn draw(&self) {
        let width = 24;
        let filled = if self.total == 0 {
            width
        } else {
            (self.current as usize * width / self.total as usize).min(width)
        };
        let empty = width - filled;
        eprint!(
            "\rImporting {} [{}{}] {}/{}",
            self.file.display(),
            "=".repeat(filled),
            " ".repeat(empty),
            self.current,
            self.total
        );
    }
}

fn collect_import_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
        } else if path.is_dir() {
            files.extend(read_csv_directory(path)?);
        } else {
            return Err(CliError::InvalidImportPath {
                path: path.display().to_string(),
            }
            .into());
        }
    }
    files.sort();
    Ok(files)
}

fn read_csv_directory(path: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path).map_err(|source| CliError::ReadImportDirectory {
        path: path.display().to_string(),
        source,
    })?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| CliError::ReadImportDirectory {
            path: path.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "csv") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

impl Display for ImportTable {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        use ImportTable::*;
        match self {
            Bar => formatter.write_str("bar"),
            Calendar => formatter.write_str("calendar"),
        }
    }
}

impl ImportMode {
    fn to_past_tense(&self) -> &'static str {
        use ImportMode::*;
        match self {
            Insert => "Inserted",
            Upsert => "Upserted",
        }
    }
}

fn parse_batch_size(value: &str) -> std::result::Result<usize, String> {
    let batch_size = value
        .parse()
        .map_err(|_| format!("invalid batch size `{value}`"))?;
    if batch_size == 0 {
        return Err("batch size must be greater than zero".into());
    }
    Ok(batch_size)
}
