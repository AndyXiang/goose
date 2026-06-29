-- Your SQL goes here
CREATE TABLE daily_bars_new (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    symbol TEXT NOT NULL,
    date TEXT NOT NULL
        CHECK (
            date = strftime('%Y-%m-%d', date)
            AND date(strftime('%Y-%m-%d', date)) = date
        ),
    open BIGINT NOT NULL,
    high BIGINT NOT NULL,
    low BIGINT NOT NULL,
    close BIGINT NOT NULL,
    volume BIGINT NOT NULL,
    amount BIGINT NOT NULL,

    UNIQUE (symbol, date),
    FOREIGN KEY (date) REFERENCES calendar(date)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,

    CHECK (low <= high),
    CHECK (open >= low AND open <= high),
    CHECK (close >= low AND close <= high)
);

INSERT INTO daily_bars_new (
    symbol, date, open, high, low, close, volume, amount
)
SELECT
    symbol, date, open, high, low, close, volume, amount
FROM daily_bars
WHERE open IS NOT NULL
  AND high IS NOT NULL
  AND low IS NOT NULL
  AND close IS NOT NULL
  AND volume IS NOT NULL
  AND amount IS NOT NULL;

DROP TABLE daily_bars;

ALTER TABLE daily_bars_new RENAME TO daily_bars;
