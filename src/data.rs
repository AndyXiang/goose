use crate::error::{Error, Result};
use chrono::NaiveDate;
use diesel::{
    AsExpression, Connection, FromSqlRow, Queryable, Selectable, SelectableHelper,
    SqliteConnection,
    connection::SimpleConnection,
    deserialize::{self, FromSql},
    prelude::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::{BigInt, Text},
    sqlite::{Sqlite, SqliteValue},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::{
    fmt,
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
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

    /// Returns whether `query_date` is open according to the trading calendar.
    ///
    /// Returns [`Error::MissingCalendarDate`] when the calendar has no entry for the date.
    pub fn is_trading_day(&mut self, query_date: Date) -> Result<bool> {
        use crate::schema::calendar;

        calendar::table
            .filter(calendar::date.eq(query_date))
            .select(calendar::is_open)
            .first::<bool>(&mut self.conn)
            .optional()?
            .ok_or_else(|| Error::MissingCalendarDate(query_date.to_string()))
    }

    /// Returns open trading dates in the inclusive interval `[start, end]`.
    ///
    /// Dates are ordered ascending. An interval with no open dates returns an empty vector.
    /// Returns [`Error::InvalidDateInterval`] when `start` is after `end`.
    pub fn get_trading_days(&mut self, start: Date, end: Date) -> Result<Vec<Date>> {
        use crate::schema::calendar;

        if start > end {
            return Err(Error::InvalidDateInterval {
                start: start.to_string(),
                end: end.to_string(),
            });
        }

        calendar::table
            .filter(calendar::date.ge(start))
            .filter(calendar::date.le(end))
            .filter(calendar::is_open.eq(true))
            .select(calendar::date)
            .order(calendar::date.asc())
            .load::<Date>(&mut self.conn)
            .map_err(Into::into)
    }

    /// Returns the earliest open trading date strictly after `query_date`.
    ///
    /// Returns [`Error::MissingTradingDayAfter`] when the calendar contains no later open date.
    pub fn first_trading_day_after(&mut self, query_date: Date) -> Result<Date> {
        use crate::schema::calendar;

        calendar::table
            .filter(calendar::date.gt(query_date))
            .filter(calendar::is_open.eq(true))
            .select(calendar::date)
            .order(calendar::date.asc())
            .first::<Date>(&mut self.conn)
            .optional()?
            .ok_or_else(|| Error::MissingTradingDayAfter(query_date.to_string()))
    }

    /// Returns the latest open trading date strictly before `query_date`.
    ///
    /// Returns [`Error::MissingTradingDayBefore`] when the calendar contains no earlier open date.
    pub fn last_trading_day_before(&mut self, query_date: Date) -> Result<Date> {
        use crate::schema::calendar;

        calendar::table
            .filter(calendar::date.lt(query_date))
            .filter(calendar::is_open.eq(true))
            .select(calendar::date)
            .order(calendar::date.desc())
            .first::<Date>(&mut self.conn)
            .optional()?
            .ok_or_else(|| Error::MissingTradingDayBefore(query_date.to_string()))
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
    /// Returns [`Error::Database`] containing Diesel's `NotFound` error when no matching bar
    /// exists.
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
            .map_err(Into::into)
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
    /// returns an empty vector. Returns [`Error::InvalidDateInterval`] when `start` is after
    /// `end`.
    pub fn get_history(
        &mut self,
        symbol: &str,
        start: Date,
        end: Date,
        adjustment: PriceAdjust,
    ) -> Result<Vec<DateBar>> {
        use crate::schema::daily_bars;

        if start > end {
            return Err(Error::InvalidDateInterval {
                start: start.to_string(),
                end: end.to_string(),
            });
        }

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
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self> {
        NaiveDate::from_ymd_opt(year, month, day)
            .map(Self)
            .ok_or_else(|| Error::InvalidDate(format!("{year:04}-{month:02}-{day:02}")))
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
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(Self)
            .map_err(|_| Error::InvalidDate(String::from(value)))
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
    type Error = Error;

    fn try_from(value: Decimal) -> Result<Self> {
        let scaled = value
            .checked_mul(Decimal::from(PRICE_MULTIPLIER))
            .ok_or_else(|| Error::InvalidData("price is too large to store".into()))?;

        if !scaled.fract().is_zero() {
            return Err(Error::InvalidData(
                "price has more than four decimal places".into(),
            ));
        }

        scaled
            .to_i64()
            .ok_or_else(|| Error::InvalidData("price is outside SQLite BIGINT range".into()))?;

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
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let value = Decimal::from_str(value)
            .map_err(|error| Error::InvalidData(format!("invalid price: {error}")))?;
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
            .ok_or_else(|| Error::InvalidData("price is outside SQLite BIGINT range".into()))?;
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
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "raw" => Ok(Self::Raw),
            "qfq" => Ok(Self::Qfq),
            "hfq" => Ok(Self::Hfq),
            value => Err(Error::InvalidData(format!(
                "unknown price adjustment: {value}"
            ))),
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

#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = crate::schema::daily_bars)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DateBar {
    pub date: Date,
    pub symbol: String,
    pub open: Option<Price>,
    pub high: Option<Price>,
    pub low: Option<Price>,
    pub close: Option<Price>,
    pub is_adjust: PriceAdjust,
}
