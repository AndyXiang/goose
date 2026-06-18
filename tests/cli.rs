use goose::data::{DataBase, Date};
use serde_json::Value;
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
    let init_json: Value = serde_json::from_str(init_stdout.trim()).unwrap();
    assert_eq!(init_json["initialized"], true);

    let calendar_stdout = assert_success(
        goose()
            .args(["db", "import"])
            .arg(&db)
            .args(["calendar"])
            .arg(&calendar_csv)
            .args(["--batch-size", "1"])
            .output()
            .unwrap(),
    );
    let calendar_json: Value = serde_json::from_str(calendar_stdout.trim()).unwrap();
    assert_eq!(calendar_json["table"], "calendar");
    assert_eq!(calendar_json["mode"], "insert");
    assert_eq!(calendar_json["affected"], 2);

    let bars_stdout = assert_success(
        goose()
            .args(["db", "import"])
            .arg(&db)
            .args(["bar"])
            .arg(&bars_csv)
            .args(["--batch-size", "1"])
            .output()
            .unwrap(),
    );
    let bars_json: Value = serde_json::from_str(bars_stdout.trim()).unwrap();
    assert_eq!(bars_json["table"], "bar");
    assert_eq!(bars_json["mode"], "insert");
    assert_eq!(bars_json["affected"], 2);

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
            .unwrap()
            .to_string(),
        "10.5000"
    );
}
