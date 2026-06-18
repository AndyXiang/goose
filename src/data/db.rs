use super::{
    decimal::Price,
    fetcher::{Fetcher, Persistable},
};
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
    sql_types::Text,
    sqlite::{Sqlite, SqliteValue},
    upsert::excluded,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::{collections::HashMap, fmt, str::FromStr};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

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

    /// Fetches batches and inserts every item, returning the total affected row count.
    ///
    /// Each successful batch is committed before the next batch is fetched. If a later fetch or
    /// insert fails, rows from earlier batches remain stored.
    pub fn insert_from<F>(&mut self, fetcher: &mut F) -> Result<usize>
    where
        F: Fetcher,
    {
        self.persist_from(fetcher, F::Item::insert_batch)
    }

    /// Fetches batches and upserts every item, returning the total affected row count.
    ///
    /// The item's [`Persistable`] implementation defines its conflict key and update behavior.
    /// Each successful batch is committed before the next batch is fetched.
    pub fn upsert_from<F>(&mut self, fetcher: &mut F) -> Result<usize>
    where
        F: Fetcher,
    {
        self.persist_from(fetcher, F::Item::upsert_batch)
    }

    fn persist_from<F, P>(&mut self, fetcher: &mut F, mut persist: P) -> Result<usize>
    where
        F: Fetcher,
        P: FnMut(&mut Self, &[F::Item]) -> Result<usize>,
    {
        let mut total = 0;
        while let Some(batch) = fetcher.fetch()? {
            if batch.is_empty() {
                return Err(FetchError::EmptyBatch.into());
            }
            total += persist(self, &batch)?;
        }
        Ok(total)
    }

    /// Returns whether `query_date` is open according to the trading calendar.
    ///
    /// Returns [`LookupError::CalendarDate`] when the calendar has no entry for the date.
    pub fn is_trading_day(&mut self, query_date: &Date) -> Result<bool> {
        use crate::schema::calendar;

        calendar::table
            .filter(calendar::date.eq(query_date))
            .select(calendar::is_open)
            .first::<bool>(&mut self.conn)
            .optional()?
            .ok_or_else(|| LookupError::CalendarDate { date: *query_date }.into())
    }

    /// Returns open trading dates in the inclusive interval `[start, end]`.
    ///
    /// Dates are ordered ascending. An interval with no open dates returns an empty vector.
    /// Panics when `start` is after `end`.
    pub fn get_trading_days(&mut self, start: &Date, end: &Date) -> Result<Vec<Date>> {
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
    pub fn next_trading_day(&mut self, query_date: &Date) -> Result<Option<Date>> {
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
    pub fn previous_trading_day(&mut self, query_date: &Date) -> Result<Option<Date>> {
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
        query_date: &Date,
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
                    date: *query_date,
                    adjustment,
                }
                .into()
            })
    }

    /// Returns all symbols with bars on `query_date` for one adjustment basis.
    ///
    /// Bars are keyed by symbol. No matches returns an empty map.
    pub fn get_cross_section(
        &mut self,
        query_date: &Date,
        adjustment: PriceAdjust,
    ) -> Result<HashMap<String, DateBar>> {
        use crate::schema::daily_bars;

        let bars = daily_bars::table
            .filter(daily_bars::date.eq(query_date))
            .filter(daily_bars::is_adjust.eq(adjustment))
            .select(DateBar::as_select())
            .load::<DateBar>(&mut self.conn)?;

        Ok(bars
            .into_iter()
            .map(|bar| (bar.symbol.clone(), bar))
            .collect())
    }

    /// Returns one symbol's bars in the inclusive interval `[start, end]`.
    ///
    /// Bars are filtered to one adjustment basis and ordered by date ascending. No matches
    /// returns an empty vector. Panics when `start` is after `end`.
    pub fn get_history(
        &mut self,
        symbol: &str,
        start: &Date,
        end: &Date,
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
}
