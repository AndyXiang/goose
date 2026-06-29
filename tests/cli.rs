use goose::data::{DataBase, Date};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("goose-cli-{name}-{}-{nonce}", std::process::id()))
}

fn goose() -> Command {
    Command::new(env!("CARGO_BIN_EXE_goose"))
}

fn assert_success(output: Output) -> String {
    if !output.status.success() {
        panic!(
            "command failed\nstatus: {}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn cli_initializes_database_and_imports_csv_files() {
    let db = temp_path("database.db");
    let calendar_csv = temp_path("calendar.csv");
    let bars_csv = temp_path("bars.csv");

    fs::write(
        &calendar_csv,
        "date,is_open\n2026-06-12,true\n2026-06-13,false\n",
    )
    .unwrap();
    fs::write(
        &bars_csv,
        "symbol,date,open,high,low,close,volume,amount\n\
         AAPL,2026-06-12,10,11,9,10.5,1000,10500\n\
         MSFT,2026-06-12,20,21,19.5,20.5,2000,41000\n",
    )
    .unwrap();

    let init_stdout = assert_success(goose().args(["db", "init"]).arg(&db).output().unwrap());
    assert!(init_stdout.contains("Initialized database at"));

    let calendar_stdout = assert_success(
        goose()
            .args(["db", "import"])
            .args(["--db"])
            .arg(&db)
            .args(["calendar"])
            .arg(&calendar_csv)
            .args(["--batch-size", "1"])
            .output()
            .unwrap(),
    );
    assert!(calendar_stdout.contains("Inserted 2 rows into calendar"));
    assert!(calendar_stdout.contains("from 1 CSV file(s)"));

    let bars_stdout = assert_success(
        goose()
            .args(["db", "import"])
            .args(["--db"])
            .arg(&db)
            .args(["bar"])
            .arg(&bars_csv)
            .args(["--batch-size", "1"])
            .output()
            .unwrap(),
    );
    assert!(bars_stdout.contains("Inserted 2 rows into bar"));
    assert!(bars_stdout.contains("from 1 CSV file(s)"));

    let mut database = DataBase::new(&db.display().to_string());
    let query_date: Date = "2026-06-12".parse().unwrap();
    assert!(database.is_trading_day(&query_date).unwrap());
    assert_eq!(database.available_symbols().unwrap(), vec!["AAPL", "MSFT"]);
    assert_eq!(
        database
            .get_bar("AAPL", &query_date)
            .unwrap()
            .ohlc
            .close
            .to_string(),
        "10.5000"
    );
}

#[test]
fn cli_import_uses_goose_db_env_without_shifting_positionals() {
    let db = temp_path("env-database.db");
    let calendar_csv = temp_path("env-calendar.csv");

    fs::write(&calendar_csv, "date,is_open\n2026-06-12,true\n").unwrap();

    assert_success(goose().args(["db", "init"]).arg(&db).output().unwrap());
    let stdout = assert_success(
        goose()
            .env("GOOSE_DB", &db)
            .args(["db", "import", "calendar"])
            .arg(&calendar_csv)
            .output()
            .unwrap(),
    );
    assert!(stdout.contains("Inserted 1 rows into calendar"));

    let mut database = DataBase::new(&db.display().to_string());
    let query_date: Date = "2026-06-12".parse().unwrap();
    assert!(database.is_trading_day(&query_date).unwrap());
}

#[test]
fn cli_import_accepts_multiple_files_and_directories() {
    let db = temp_path("multi-database.db");
    let calendar_one = temp_path("calendar-one.csv");
    let calendar_two = temp_path("calendar-two.csv");
    let bars_dir = temp_path("bars-dir");
    let ignored_txt = bars_dir.join("ignored.txt");
    let bars_one = bars_dir.join("bars-one.csv");
    let bars_two = bars_dir.join("bars-two.csv");

    fs::write(&calendar_one, "date,is_open\n2026-06-12,true\n").unwrap();
    fs::write(
        &calendar_two,
        "date,is_open\n2026-06-13,false\n2026-06-15,true\n",
    )
    .unwrap();
    fs::create_dir(&bars_dir).unwrap();
    fs::write(&ignored_txt, "not,a,csv\n").unwrap();
    fs::write(
        &bars_one,
        "symbol,date,open,high,low,close,volume,amount\n\
         AAPL,2026-06-12,10,11,9,10.5,1000,10500\n",
    )
    .unwrap();
    fs::write(
        &bars_two,
        "symbol,date,open,high,low,close,volume,amount\n\
         MSFT,2026-06-15,20,21,19.5,20.5,2000,41000\n",
    )
    .unwrap();

    assert_success(goose().args(["db", "init"]).arg(&db).output().unwrap());

    let calendar_stdout = assert_success(
        goose()
            .args(["db", "import", "--db"])
            .arg(&db)
            .arg("calendar")
            .arg(&calendar_one)
            .arg(&calendar_two)
            .output()
            .unwrap(),
    );
    assert!(calendar_stdout.contains("Inserted 3 rows into calendar"));
    assert!(calendar_stdout.contains("from 2 CSV file(s)"));

    let bars_stdout = assert_success(
        goose()
            .args(["db", "import", "--db"])
            .arg(&db)
            .arg("bar")
            .arg(&bars_dir)
            .output()
            .unwrap(),
    );
    assert!(bars_stdout.contains("Inserted 2 rows into bar"));
    assert!(bars_stdout.contains("from 2 CSV file(s)"));

    let mut database = DataBase::new(&db.display().to_string());
    assert_eq!(database.available_symbols().unwrap(), vec!["AAPL", "MSFT"]);
    assert!(
        database
            .get_bar("AAPL", &"2026-06-12".parse().unwrap())
            .is_ok()
    );
    assert!(
        database
            .get_bar("MSFT", &"2026-06-15".parse().unwrap())
            .is_ok()
    );
}

#[test]
fn cli_import_skips_invalid_bar_data_and_continues() {
    let db = temp_path("skipped-bars-database.db");
    let calendar_csv = temp_path("skipped-bars-calendar.csv");
    let bars_csv = temp_path("skipped-bars.csv");

    fs::write(
        &calendar_csv,
        "date,is_open\n2026-06-12,true\n2026-06-15,true\n",
    )
    .unwrap();
    fs::write(
        &bars_csv,
        "symbol,date,open,high,low,close,volume,amount\n\
         AAPL,2026-06-12,10,11,9,10.5,1000,10500\n\
         MSFT,2026-06-15,20,19,21,20.5,2000,41000\n",
    )
    .unwrap();

    assert_success(goose().args(["db", "init"]).arg(&db).output().unwrap());
    assert_success(
        goose()
            .args(["db", "import", "--db"])
            .arg(&db)
            .arg("calendar")
            .arg(&calendar_csv)
            .output()
            .unwrap(),
    );
    let stdout = assert_success(
        goose()
            .args(["db", "import", "--db"])
            .arg(&db)
            .arg("bar")
            .arg(&bars_csv)
            .output()
            .unwrap(),
    );

    assert!(stdout.contains("Inserted 1 rows into bar"));

    let mut database = DataBase::new(&db.display().to_string());
    let valid_date: Date = "2026-06-12".parse().unwrap();
    let skipped_date: Date = "2026-06-15".parse().unwrap();
    assert_eq!(
        database
            .get_bar("AAPL", &valid_date)
            .unwrap()
            .ohlc
            .close
            .to_string(),
        "10.5000"
    );
    assert!(database.get_bar("MSFT", &skipped_date).is_err());
}
