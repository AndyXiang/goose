use crate::error::Error;
use chrono::NaiveDate;
use diesel::{
    AsExpression, Connection, FromSqlRow, Queryable, Selectable, SqliteConnection,
    deserialize::{self, FromSql},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::{BigInt, Text},
    sqlite::{Sqlite, SqliteValue},
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::{
    fmt,
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    str::FromStr,
};

pub struct DataBase {
    pub conn: SqliteConnection,
}

impl DataBase {
    pub fn new(path: &str) -> Self {
        let conn = SqliteConnection::establish(path)
            .unwrap_or_else(|_| panic!("Fail connecting to {}", path));
        Self { conn }
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
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self, Error> {
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

    fn from_str(value: &str) -> Result<Self, Self::Err> {
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

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
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

    fn from_str(value: &str) -> Result<Self, Self::Err> {
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

    fn from_str(value: &str) -> Result<Self, Self::Err> {
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

#[derive(Queryable, Selectable)]
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
