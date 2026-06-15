use crate::{
    error::{FetchError, LookupError, Result, ValidationError},
    schema,
};
use chrono::NaiveDate;
use diesel::{
    AsExpression, Connection, FromSqlRow, Insertable, Queryable, Selectable, SelectableHelper,
    SqliteConnection,
    connection::SimpleConnection,
    deserialize::{self, FromSql},
    prelude::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::{BigInt, Text},
    sqlite::{Sqlite, SqliteValue},
    upsert::excluded,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::{
    fmt,
    fs::File,
    io::Read,
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    path::Path,
    str::FromStr,
};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub struct DataBase {
    pub conn: SqliteConnection,
}

impl DataBase {
    pub fn new(path: &str) -> Self {
        let mut conn = SqliteConnection::establish(path)
            .unwrap_or_else(|_| panic!("Fail connecting to {}", path));

        conn.run_pending_migrations(MIGRATIONS)
            .unwrap_or_else(|error| panic!("Fail running database migrations: {error}"));
        conn.batch_execute("PRAGMA foreign_keys = ON;")
            .unwrap_or_else(|error| panic!("Fail enabling SQLite foreign keys: {error}"));

        Self { conn }
    }

    /// Inserts calendar entries and returns the number of inserted rows.
    ///
    /// Duplicate dates produce a database constraint error. Empty input returns zero.
    pub fn insert_calendar(&mut self, entries: &[CalendarEntry]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        diesel::insert_into(schema::calendar::table)
            .values(entries)
            .execute(&mut self.conn)
            .map_err(Into::into)
    }

    /// Inserts calendar entries or updates `is_open` when a date already exists.
    ///
    /// Empty input returns zero.
    pub fn upsert_calendar(&mut self, entries: &[CalendarEntry]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        self.conn
            .transaction::<usize, diesel::result::Error, _>(|conn| {
                entries.iter().try_fold(0, |total, entry| {
                    diesel::insert_into(schema::calendar::table)
                        .values(entry)
                        .on_conflict(schema::calendar::date)
                        .do_update()
                        .set(schema::calendar::is_open.eq(excluded(schema::calendar::is_open)))
                        .execute(conn)
                        .map(|count| total + count)
                })
            })
            .map_err(Into::into)
    }

    /// Inserts bars and returns the number of inserted rows.
    ///
    /// Duplicate `(symbol, date, is_adjust)` keys produce a database constraint error. Calendar
    /// entries for all bar dates must already exist. Empty input returns zero.
    pub fn insert_bars(&mut self, bars: &[DateBar]) -> Result<usize> {
        if bars.is_empty() {
            return Ok(0);
        }

        diesel::insert_into(schema::daily_bars::table)
            .values(bars)
            .execute(&mut self.conn)
            .map_err(Into::into)
    }

    /// Inserts bars or updates OHLC values when their business key already exists.
    ///
    /// The conflict key is `(symbol, date, is_adjust)`. Calendar entries for all bar dates must
    /// already exist. Empty input returns zero.
    pub fn upsert_bars(&mut self, bars: &[DateBar]) -> Result<usize> {
        if bars.is_empty() {
            return Ok(0);
        }

        self.conn
            .transaction::<usize, diesel::result::Error, _>(|conn| {
                bars.iter().try_fold(0, |total, bar| {
                    diesel::insert_into(schema::daily_bars::table)
                        .values(bar)
                        .on_conflict((
                            schema::daily_bars::symbol,
                            schema::daily_bars::date,
                            schema::daily_bars::is_adjust,
                        ))
                        .do_update()
                        .set((
                            schema::daily_bars::open.eq(excluded(schema::daily_bars::open)),
                            schema::daily_bars::high.eq(excluded(schema::daily_bars::high)),
                            schema::daily_bars::low.eq(excluded(schema::daily_bars::low)),
                            schema::daily_bars::close.eq(excluded(schema::daily_bars::close)),
                        ))
                        .execute(conn)
                        .map(|count| total + count)
                })
            })
            .map_err(Into::into)
    }

    /// Returns whether `query_date` is open according to the trading calendar.
    ///
    /// Returns [`LookupError::CalendarDate`] when the calendar has no entry for the date.
    pub fn is_trading_day(&mut self, query_date: Date) -> Result<bool> {
        use crate::schema::calendar;

        calendar::table
            .filter(calendar::date.eq(query_date))
            .select(calendar::is_open)
            .first::<bool>(&mut self.conn)
            .optional()?
            .ok_or_else(|| LookupError::CalendarDate { date: query_date }.into())
    }

    /// Returns open trading dates in the inclusive interval `[start, end]`.
    ///
    /// Dates are ordered ascending. An interval with no open dates returns an empty vector.
    /// Panics when `start` is after `end`.
    pub fn get_trading_days(&mut self, start: Date, end: Date) -> Result<Vec<Date>> {
        use crate::schema::calendar;

        assert!(start <= end, "start date {start} is after end date {end}");

        calendar::table
            .filter(calendar::date.ge(start))
            .filter(calendar::date.le(end))
            .filter(calendar::is_open.eq(true))
            .select(calendar::date)
            .order(calendar::date.asc())
            .load::<Date>(&mut self.conn)
            .map_err(Into::into)
    }

    /// Returns the earliest open trading date strictly after `query_date`, if one exists.
    pub fn first_trading_day_after(&mut self, query_date: Date) -> Result<Option<Date>> {
        use crate::schema::calendar;

        calendar::table
            .filter(calendar::date.gt(query_date))
            .filter(calendar::is_open.eq(true))
            .select(calendar::date)
            .order(calendar::date.asc())
            .first::<Date>(&mut self.conn)
            .optional()
            .map_err(Into::into)
    }

    /// Returns the latest open trading date strictly before `query_date`, if one exists.
    pub fn last_trading_day_before(&mut self, query_date: Date) -> Result<Option<Date>> {
        use crate::schema::calendar;

        calendar::table
            .filter(calendar::date.lt(query_date))
            .filter(calendar::is_open.eq(true))
            .select(calendar::date)
            .order(calendar::date.desc())
            .first::<Date>(&mut self.conn)
            .optional()
            .map_err(Into::into)
    }

    /// Returns all distinct symbols that have stored bars, ordered ascending.
    pub fn available_symbols(&mut self) -> Result<Vec<String>> {
        use crate::schema::daily_bars;

        daily_bars::table
            .select(daily_bars::symbol)
            .distinct()
            .order(daily_bars::symbol.asc())
            .load::<String>(&mut self.conn)
            .map_err(Into::into)
    }

    /// Returns one bar identified by symbol, date, and adjustment basis.
    ///
    /// Returns [`LookupError::Bar`] when no matching bar exists.
    pub fn get_bar(
        &mut self,
        symbol: &str,
        query_date: Date,
        adjustment: PriceAdjust,
    ) -> Result<DateBar> {
        use crate::schema::daily_bars;

        daily_bars::table
            .filter(daily_bars::symbol.eq(symbol))
            .filter(daily_bars::date.eq(query_date))
            .filter(daily_bars::is_adjust.eq(adjustment))
            .select(DateBar::as_select())
            .first::<DateBar>(&mut self.conn)
            .optional()?
            .ok_or_else(|| {
                LookupError::Bar {
                    symbol: symbol.to_owned(),
                    date: query_date,
                    adjustment,
                }
                .into()
            })
    }

    /// Returns all symbols with bars on `query_date` for one adjustment basis.
    ///
    /// Bars are ordered by symbol. No matches returns an empty vector.
    pub fn get_cross_section(
        &mut self,
        query_date: Date,
        adjustment: PriceAdjust,
    ) -> Result<Vec<DateBar>> {
        use crate::schema::daily_bars;

        daily_bars::table
            .filter(daily_bars::date.eq(query_date))
            .filter(daily_bars::is_adjust.eq(adjustment))
            .select(DateBar::as_select())
            .order(daily_bars::symbol.asc())
            .load::<DateBar>(&mut self.conn)
            .map_err(Into::into)
    }

    /// Returns one symbol's bars in the inclusive interval `[start, end]`.
    ///
    /// Bars are filtered to one adjustment basis and ordered by date ascending. No matches
    /// returns an empty vector. Panics when `start` is after `end`.
    pub fn get_history(
        &mut self,
        symbol: &str,
        start: Date,
        end: Date,
        adjustment: PriceAdjust,
    ) -> Result<Vec<DateBar>> {
        use crate::schema::daily_bars;

        assert!(start <= end, "start date {start} is after end date {end}");

        daily_bars::table
            .filter(daily_bars::symbol.eq(symbol))
            .filter(daily_bars::date.ge(start))
            .filter(daily_bars::date.le(end))
            .filter(daily_bars::is_adjust.eq(adjustment))
            .select(DateBar::as_select())
            .order(daily_bars::date.asc())
            .load::<DateBar>(&mut self.conn)
            .map_err(Into::into)
    }
    //
    // pub fn new_in_memory() -> Self {
    //     let conn = SqliteConnection::establish(":memory:").unwrap();
    //     Self { conn }
    // }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct Date(NaiveDate);

impl Date {
    pub fn from_ymd(year: i32, month: u32, day: u32) -> std::result::Result<Self, ValidationError> {
        NaiveDate::from_ymd_opt(year, month, day)
            .map(Self)
            .ok_or_else(|| ValidationError::Date {
                value: format!("{year:04}-{month:02}-{day:02}"),
            })
    }

    pub const fn as_naive_date(&self) -> &NaiveDate {
        &self.0
    }

    pub const fn into_naive_date(self) -> NaiveDate {
        self.0
    }
}

impl From<NaiveDate> for Date {
    fn from(value: NaiveDate) -> Self {
        Self(value)
    }
}

impl From<Date> for NaiveDate {
    fn from(value: Date) -> Self {
        value.0
    }
}

impl AsRef<NaiveDate> for Date {
    fn as_ref(&self) -> &NaiveDate {
        self.as_naive_date()
    }
}

impl FromStr for Date {
    type Err = ValidationError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(Self)
            .map_err(|_| ValidationError::Date {
                value: value.to_owned(),
            })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.format("%Y-%m-%d").fmt(formatter)
    }
}

impl FromSql<Text, Sqlite> for Date {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<Text, Sqlite>>::from_sql(value)?;
        Ok(value.parse()?)
    }
}

impl ToSql<Text, Sqlite> for Date {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

const PRICE_SCALE: u32 = 4;
const PRICE_MULTIPLIER: i64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct Price(Decimal);

impl Price {
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0
            .checked_add(rhs.0)
            .and_then(|value| value.try_into().ok())
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0
            .checked_sub(rhs.0)
            .and_then(|value| value.try_into().ok())
    }

    pub fn checked_mul(self, rhs: Decimal) -> Option<Self> {
        self.0
            .checked_mul(rhs)
            .and_then(|value| value.try_into().ok())
    }

    pub fn checked_div(self, rhs: Decimal) -> Option<Self> {
        self.0
            .checked_div(rhs)
            .and_then(|value| value.try_into().ok())
    }

    pub fn checked_ratio(self, rhs: Self) -> Option<Decimal> {
        self.0.checked_div(rhs.0)
    }

    fn as_stored_integer(self) -> Option<i64> {
        self.0
            .checked_mul(Decimal::from(PRICE_MULTIPLIER))?
            .to_i64()
    }
}

impl TryFrom<Decimal> for Price {
    type Error = ValidationError;

    fn try_from(value: Decimal) -> std::result::Result<Self, Self::Error> {
        let scaled = value
            .checked_mul(Decimal::from(PRICE_MULTIPLIER))
            .ok_or_else(|| ValidationError::Price {
                reason: "value is too large to store".into(),
            })?;

        if !scaled.fract().is_zero() {
            return Err(ValidationError::Price {
                reason: "value has more than four decimal places".into(),
            });
        }

        scaled.to_i64().ok_or_else(|| ValidationError::Price {
            reason: "value is outside SQLite BIGINT range".into(),
        })?;

        let mut value = value;
        value.rescale(PRICE_SCALE);
        Ok(Self(value))
    }
}

impl From<Price> for Decimal {
    fn from(value: Price) -> Self {
        value.0
    }
}

impl FromStr for Price {
    type Err = ValidationError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let value = Decimal::from_str(value).map_err(|error| ValidationError::Price {
            reason: error.to_string(),
        })?;
        Self::try_from(value)
    }
}

impl fmt::Display for Price {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Add for Price {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("price addition overflow")
    }
}

impl Sub for Price {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("price subtraction overflow")
    }
}

impl Mul<Decimal> for Price {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        self.checked_mul(rhs)
            .expect("price multiplication produced an invalid price")
    }
}

impl Div<Decimal> for Price {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        self.checked_div(rhs)
            .expect("price division produced an invalid price")
    }
}

impl Div for Price {
    type Output = Decimal;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_ratio(rhs).expect("price division failed")
    }
}

impl AddAssign for Price {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Price {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl FromSql<BigInt, Sqlite> for Price {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let value = i64::from_sql(value)?;
        Ok(Self(Decimal::from_i128_with_scale(
            i128::from(value),
            PRICE_SCALE,
        )))
    }
}

impl ToSql<BigInt, Sqlite> for Price {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let value = self
            .as_stored_integer()
            .ok_or_else(|| ValidationError::Price {
                reason: "value is outside SQLite BIGINT range".into(),
            })?;
        out.set_value(value);
        Ok(IsNull::No)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub enum PriceAdjust {
    Raw,
    Qfq,
    Hfq,
}

impl FromStr for PriceAdjust {
    type Err = ValidationError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "raw" => Ok(Self::Raw),
            "qfq" => Ok(Self::Qfq),
            "hfq" => Ok(Self::Hfq),
            value => Err(ValidationError::PriceAdjust {
                value: value.to_owned(),
            }),
        }
    }
}

impl fmt::Display for PriceAdjust {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Raw => "raw",
            Self::Qfq => "qfq",
            Self::Hfq => "hfq",
        };
        formatter.write_str(value)
    }
}

impl FromSql<Text, Sqlite> for PriceAdjust {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        Ok(<String as FromSql<Text, Sqlite>>::from_sql(value)?.parse()?)
    }
}

impl ToSql<Text, Sqlite> for PriceAdjust {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_string());
        Ok(IsNull::No)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::daily_bars)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(treat_none_as_default_value = false)]
pub struct DateBar {
    pub date: Date,
    pub symbol: String,
    pub open: Option<Price>,
    pub high: Option<Price>,
    pub low: Option<Price>,
    pub close: Option<Price>,
    pub is_adjust: PriceAdjust,
}

impl DateBar {
    /// Creates a bar after validating its symbol and OHLC relationships.
    ///
    /// Empty symbols are rejected. When the relevant prices are present, `low` must not exceed
    /// `high`, and `open` and `close` must lie within the inclusive `[low, high]` range.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        symbol: impl Into<String>,
        date: Date,
        adjustment: PriceAdjust,
        open: Option<Price>,
        high: Option<Price>,
        low: Option<Price>,
        close: Option<Price>,
    ) -> std::result::Result<Self, ValidationError> {
        let symbol = symbol.into();
        if symbol.trim().is_empty() {
            return Err(ValidationError::EmptySymbol);
        }

        validate_ohlc(open, high, low, close)?;
        Ok(Self {
            symbol,
            date,
            is_adjust: adjustment,
            open,
            high,
            low,
            close,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Insertable)]
#[diesel(table_name = schema::calendar)]
pub struct CalendarEntry {
    pub date: Date,
    pub is_open: bool,
}

pub trait Fetcher {
    type Item;
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
    /// The CSV must contain the headers `symbol,date,is_adjust,open,high,low,close` in that
    /// order. Empty OHLC fields are returned as `None`.
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

        const HEADERS: [&str; 7] = [
            "symbol",
            "date",
            "is_adjust",
            "open",
            "high",
            "low",
            "close",
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

    fn parse_record(record: &csv::StringRecord) -> Result<DateBar> {
        let row = record.position().map_or(0, csv::Position::line);
        let parse_price = |index: usize, name: &'static str| -> Result<Option<Price>> {
            let value = csv_field(record, row, index, name)?;
            if value.is_empty() {
                return Ok(None);
            }
            value
                .parse()
                .map(Some)
                .map_err(|source| FetchError::InvalidRecord { row, source }.into())
        };

        DateBar::new(
            csv_field(record, row, 0, "symbol")?,
            csv_field(record, row, 1, "date")?
                .parse()
                .map_err(|source| FetchError::InvalidRecord { row, source })?,
            csv_field(record, row, 2, "is_adjust")?
                .parse()
                .map_err(|source| FetchError::InvalidRecord { row, source })?,
            parse_price(3, "open")?,
            parse_price(4, "high")?,
            parse_price(5, "low")?,
            parse_price(6, "close")?,
        )
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
            batch.push(Self::parse_record(&record)?);
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

fn validate_ohlc(
    open: Option<Price>,
    high: Option<Price>,
    low: Option<Price>,
    close: Option<Price>,
) -> std::result::Result<(), ValidationError> {
    if let (Some(low), Some(high)) = (low, high)
        && low > high
    {
        return Err(ValidationError::Ohlc {
            reason: "`low` must not be greater than `high`".into(),
        });
    }

    for (name, value) in [("open", open), ("close", close)] {
        if let (Some(value), Some(low)) = (value, low)
            && value < low
        {
            return Err(ValidationError::Ohlc {
                reason: format!("`{name}` must not be below `low`"),
            });
        }
        if let (Some(value), Some(high)) = (value, high)
            && value > high
        {
            return Err(ValidationError::Ohlc {
                reason: format!("`{name}` must not be above `high`"),
            });
        }
    }

    Ok(())
}
