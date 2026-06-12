-- Your SQL goes here
CREATE TABLE daily_bars (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    symbol INTEGER NOT NULL,
    date TEXT NOT NULL,
    open INTEGER,
    high INTEGER,
    low INTEGER,
    close INTEGER,
    is_adjust TEXT NOT NULL
);

CREATE TABLE calendar (
    date TEXT NOT NULL PRIMARY KEY,
    market TEXT NOT NULL,
    is_open INTEGER NOT NULL
);

