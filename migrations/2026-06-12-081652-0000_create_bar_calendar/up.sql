-- Your SQL goes here
CREATE TABLE calendar (
    date TEXT NOT NULL PRIMARY KEY
        CHECK (
            date = strftime('%Y-%m-%d', date)
            AND date(strftime('%Y-%m-%d', date)) = date
        ),
    is_open BOOLEAN NOT NULL
);

CREATE TABLE daily_bars (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    date TEXT NOT NULL
        CHECK (
            date = strftime('%Y-%m-%d', date)
            AND date(strftime('%Y-%m-%d', date)) = date
        ),
    open BIGINT,
    high BIGINT,
    low BIGINT,
    close BIGINT,
    volume BIGINT,
    amount BIGINT,

    UNIQUE (symbol, date),
    FOREIGN KEY (date) REFERENCES calendar(date)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,

    CHECK (low IS NULL OR high IS NULL OR low <= high),
    CHECK (open IS NULL OR low IS NULL OR open >= low),
    CHECK (open IS NULL OR high IS NULL OR open <= high),
    CHECK (close IS NULL OR low IS NULL OR close >= low),
    CHECK (close IS NULL OR high IS NULL OR close <= high)
);
