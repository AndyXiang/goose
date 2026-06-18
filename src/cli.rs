use crate::data::{CsvBarFetcher, CsvCalendarFetcher, DataBase};
use crate::error::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
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
        #[arg(value_name = "DB", env = "GOOSE_DB")]
        db: PathBuf,
        #[arg(value_name = "TABLE")]
        table: ImportTable,
        #[arg(value_name = "FILE")]
        file: PathBuf,
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
                file,
                mode,
                batch_size,
            } => import_csv(db, table, file, mode, *batch_size),
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

fn init_database(path: &PathBuf) -> Result<()> {
    let _database = DataBase::new(&path.display().to_string());
    println!(
        "{}",
        serde_json::json!({
            "database": path,
            "initialized": true,
        })
    );
    Ok(())
}

fn import_csv(
    db: &PathBuf,
    table: &ImportTable,
    file: &PathBuf,
    mode: &ImportMode,
    batch_size: usize,
) -> Result<()> {
    let mut database = DataBase::new(&db.display().to_string());
    let affected = match table {
        ImportTable::Bar => {
            let mut fetcher = CsvBarFetcher::from_path(file, batch_size)?;
            persist_csv(&mut database, &mut fetcher, mode)?
        }
        ImportTable::Calendar => {
            let mut fetcher = CsvCalendarFetcher::from_path(file, batch_size)?;
            persist_csv(&mut database, &mut fetcher, mode)?
        }
    };

    println!(
        "{}",
        serde_json::json!({
            "database": db,
            "file": file,
            "table": table.to_string(),
            "mode": mode.to_string(),
            "affected": affected,
        })
    );
    Ok(())
}

fn persist_csv<F>(database: &mut DataBase, fetcher: &mut F, mode: &ImportMode) -> Result<usize>
where
    F: crate::data::Fetcher,
{
    match mode {
        ImportMode::Insert => database.insert_from(fetcher),
        ImportMode::Upsert => database.upsert_from(fetcher),
    }
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

fn parse_batch_size(value: &str) -> std::result::Result<usize, String> {
    let batch_size = value
        .parse()
        .map_err(|_| format!("invalid batch size `{value}`"))?;
    if batch_size == 0 {
        return Err("batch size must be greater than zero".into());
    }
    Ok(batch_size)
}
