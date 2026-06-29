-- This file should undo anything in `up.sql`
CREATE TABLE daily_bars_old (
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

INSERT INTO daily_bars_old (
  symbol, date, open, high, low, close, volume, amount
)
SELECT
  symbol, date, open, high, low, close, volume, amount
FROM daily_bars;

DROP TABLE daily_bars;

ALTER TABLE daily_bars_old RENAME TO daily_bars;
