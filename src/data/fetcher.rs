use super::{
    db::{CalendarEntry, DataBase, DateBar, Ohlc},
    decimal::{Price, Quantity},
};
use crate::error::{FetchError, Result};
use std::{fs::File, io::Read, path::Path};

/// A fetched model that can be persisted by [`DataBase`].
///
/// This bound is stronger than Diesel's [`Insertable`] derive because an upsert also needs a
/// model-specific conflict key and update policy.
pub trait Persistable: Sized {
    fn insert_batch(database: &mut DataBase, items: &[Self]) -> Result<usize>;
    fn upsert_batch(database: &mut DataBase, items: &[Self]) -> Result<usize>;
}

impl Persistable for DateBar {
    fn insert_batch(database: &mut DataBase, items: &[Self]) -> Result<usize> {
        database.insert_bars(items)
    }

    fn upsert_batch(database: &mut DataBase, items: &[Self]) -> Result<usize> {
        database.upsert_bars(items)
    }
}

impl Persistable for CalendarEntry {
    fn insert_batch(database: &mut DataBase, items: &[Self]) -> Result<usize> {
        database.insert_calendar(items)
    }

    fn upsert_batch(database: &mut DataBase, items: &[Self]) -> Result<usize> {
        database.upsert_calendar(items)
    }
}

pub trait Fetcher {
    type Item: Persistable;

    /// Returns the next non-empty batch, or `None` when the source is exhausted.
    fn fetch(&mut self) -> Result<Option<Vec<Self::Item>>>;
}

pub struct CsvBarFetcher<R: Read> {
    reader: csv::Reader<R>,
    batch_size: usize,
    finished: bool,
}

impl<R: Read> CsvBarFetcher<R> {
    /// Creates a batched bar fetcher from a CSV reader.
    ///
    /// The CSV must contain the headers `symbol,date,open,high,low,close,volume,amount` in that
    /// order.
    /// Rows with empty or invalid bar payload fields are skipped. Invalid symbol or date values are
    /// still treated as input errors because they identify the bar.
    pub fn from_reader(reader: R, batch_size: usize) -> Result<Self> {
        let reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(reader);
        Self::from_csv_reader(reader, batch_size)
    }

    fn from_csv_reader(mut reader: csv::Reader<R>, batch_size: usize) -> Result<Self> {
        assert!(
            batch_size > 0,
            "CSV fetcher batch size must be greater than zero"
        );

        const HEADERS: [&str; 8] = [
            "symbol", "date", "open", "high", "low", "close", "volume", "amount",
        ];
        let headers = reader.headers()?;
        if !headers.iter().eq(HEADERS) {
            return Err(FetchError::InvalidHeaders {
                expected: HEADERS.join(","),
                actual: headers.iter().collect::<Vec<_>>().join(","),
            }
            .into());
        }

        Ok(Self {
            reader,
            batch_size,
            finished: false,
        })
    }

    fn parse_record(record: &csv::StringRecord) -> Result<Option<DateBar>> {
        let row = record.position().map_or(0, csv::Position::line);
        let symbol = csv_field(record, row, 0, "symbol")?;
        let date = csv_field(record, row, 1, "date")?
            .parse()
            .map_err(|source| FetchError::InvalidRecord { row, source })?;

        let Some((ohlc, volume, amount)) = parse_bar_payload(record) else {
            return Ok(None);
        };

        DateBar::new(symbol, date, ohlc, volume, amount)
            .map(Some)
            .map_err(|source| FetchError::InvalidRecord { row, source }.into())
    }
}

impl CsvBarFetcher<File> {
    /// Opens a CSV file and creates a batched bar fetcher.
    pub fn from_path(path: impl AsRef<Path>, batch_size: usize) -> Result<Self> {
        let reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_path(path)?;
        Self::from_csv_reader(reader, batch_size)
    }
}

impl<R: Read> Fetcher for CsvBarFetcher<R> {
    type Item = DateBar;

    fn fetch(&mut self) -> Result<Option<Vec<Self::Item>>> {
        if self.finished {
            return Ok(None);
        }

        let mut batch = Vec::with_capacity(self.batch_size);
        let mut record = csv::StringRecord::new();
        while batch.len() < self.batch_size {
            if !self.reader.read_record(&mut record)? {
                self.finished = true;
                break;
            }
            if let Some(bar) = Self::parse_record(&record)? {
                batch.push(bar);
            }
        }

        if batch.is_empty() {
            Ok(None)
        } else {
            Ok(Some(batch))
        }
    }
}

pub struct CsvCalendarFetcher<R: Read> {
    reader: csv::Reader<R>,
    batch_size: usize,
    finished: bool,
}

impl<R: Read> CsvCalendarFetcher<R> {
    /// Creates a batched calendar fetcher from a CSV reader.
    ///
    /// The CSV must contain the headers `date,is_open` in that order. `is_open` accepts
    /// `true`, `false`, `1`, and `0`; word values are case-insensitive.
    pub fn from_reader(reader: R, batch_size: usize) -> Result<Self> {
        let reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(reader);
        Self::from_csv_reader(reader, batch_size)
    }

    fn from_csv_reader(mut reader: csv::Reader<R>, batch_size: usize) -> Result<Self> {
        assert!(
            batch_size > 0,
            "CSV fetcher batch size must be greater than zero"
        );

        const HEADERS: [&str; 2] = ["date", "is_open"];
        let headers = reader.headers()?;
        if !headers.iter().eq(HEADERS) {
            return Err(FetchError::InvalidHeaders {
                expected: HEADERS.join(","),
                actual: headers.iter().collect::<Vec<_>>().join(","),
            }
            .into());
        }

        Ok(Self {
            reader,
            batch_size,
            finished: false,
        })
    }

    fn parse_record(record: &csv::StringRecord) -> Result<CalendarEntry> {
        let row = record.position().map_or(0, csv::Position::line);
        let date = csv_field(record, row, 0, "date")?
            .parse()
            .map_err(|source| FetchError::InvalidRecord { row, source })?;
        let is_open = match csv_field(record, row, 1, "is_open")? {
            "1" => true,
            "0" => false,
            value if value.eq_ignore_ascii_case("true") => true,
            value if value.eq_ignore_ascii_case("false") => false,
            value => {
                return Err(FetchError::InvalidField {
                    row,
                    field: "is_open",
                    value: value.to_owned(),
                }
                .into());
            }
        };

        Ok(CalendarEntry { date, is_open })
    }
}

impl CsvCalendarFetcher<File> {
    /// Opens a CSV file and creates a batched calendar fetcher.
    pub fn from_path(path: impl AsRef<Path>, batch_size: usize) -> Result<Self> {
        let reader = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_path(path)?;
        Self::from_csv_reader(reader, batch_size)
    }
}

impl<R: Read> Fetcher for CsvCalendarFetcher<R> {
    type Item = CalendarEntry;

    fn fetch(&mut self) -> Result<Option<Vec<Self::Item>>> {
        if self.finished {
            return Ok(None);
        }

        let mut batch = Vec::with_capacity(self.batch_size);
        let mut record = csv::StringRecord::new();
        while batch.len() < self.batch_size {
            if !self.reader.read_record(&mut record)? {
                self.finished = true;
                break;
            }
            batch.push(Self::parse_record(&record)?);
        }

        if batch.is_empty() {
            Ok(None)
        } else {
            Ok(Some(batch))
        }
    }
}

fn csv_field<'a>(
    record: &'a csv::StringRecord,
    row: u64,
    index: usize,
    name: &'static str,
) -> Result<&'a str> {
    record
        .get(index)
        .ok_or_else(|| FetchError::MissingField { row, field: name }.into())
}

fn required_payload_field<T>(record: &csv::StringRecord, index: usize) -> Option<T>
where
    T: std::str::FromStr,
{
    let value = record.get(index)?;
    if value.is_empty() {
        return None;
    }
    value.parse().ok()
}

fn parse_bar_payload(record: &csv::StringRecord) -> Option<(Ohlc, Quantity, Price)> {
    let open = required_payload_field(record, 2)?;
    let high = required_payload_field(record, 3)?;
    let low = required_payload_field(record, 4)?;
    let close = required_payload_field(record, 5)?;
    let volume = required_payload_field(record, 6)?;
    let amount = required_payload_field(record, 7)?;
    let ohlc = Ohlc::new(open, high, low, close).ok()?;
    Some((ohlc, volume, amount))
}
