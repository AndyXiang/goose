use crate::error::ValidationError;
use diesel::{
    AsExpression, FromSqlRow,
    deserialize::{self, FromSql},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::BigInt,
    sqlite::{Sqlite, SqliteValue},
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::{
    fmt,
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    str::FromStr,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct Price(Decimal);

impl Price {
    pub const SCALE: u32 = 4;
    pub const MULTIPLIER: i64 = 10_000;

    pub fn is_negative(self) -> bool {
        self.0 < Decimal::ZERO
    }

    pub fn is_positive(self) -> bool {
        self.0 > Decimal::ZERO
    }

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

    pub fn checked_mul_quantity(self, rhs: Quantity) -> Option<Self> {
        self.0
            .checked_mul(Decimal::from(rhs))
            .and_then(|value| value.try_into().ok())
    }

    pub fn checked_div_quantity(self, rhs: Quantity) -> Option<Self> {
        self.0
            .checked_div(Decimal::from(rhs))
            .and_then(|value| value.try_into().ok())
    }

    fn as_stored_integer(self) -> Option<i64> {
        self.0
            .checked_mul(Decimal::from(Self::MULTIPLIER))?
            .to_i64()
    }
}

impl TryFrom<Decimal> for Price {
    type Error = ValidationError;

    fn try_from(value: Decimal) -> std::result::Result<Self, Self::Error> {
        let scaled = value
            .checked_mul(Decimal::from(Self::MULTIPLIER))
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
        value.rescale(Self::SCALE);
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

impl Mul<Quantity> for Price {
    type Output = Self;

    fn mul(self, rhs: Quantity) -> Self::Output {
        self.checked_mul_quantity(rhs)
            .expect("price and quantity multiplication produced an invalid price")
    }
}

impl Div<Decimal> for Price {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        self.checked_div(rhs)
            .expect("price division produced an invalid price")
    }
}

impl Div<Quantity> for Price {
    type Output = Self;

    fn div(self, rhs: Quantity) -> Self::Output {
        self.checked_div_quantity(rhs)
            .expect("price and quantity division produced an invalid price")
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
            Self::SCALE,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, AsExpression, FromSqlRow)]
#[diesel(sql_type = BigInt)]
pub struct Quantity(Decimal);

impl Quantity {
    pub const SCALE: u32 = 4;

    pub const MULTIPLIER: i64 = 10_000;

    pub fn is_negative(self) -> bool {
        self.0 < Decimal::ZERO
    }

    pub fn is_positive(self) -> bool {
        self.0 > Decimal::ZERO
    }

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

    pub fn checked_mul_price(self, rhs: Price) -> Option<Price> {
        rhs.checked_mul_quantity(self)
    }

    fn as_stored_integer(self) -> Option<i64> {
        self.0
            .checked_mul(Decimal::from(Self::MULTIPLIER))?
            .to_i64()
    }
}

impl TryFrom<Decimal> for Quantity {
    type Error = ValidationError;

    fn try_from(value: Decimal) -> std::result::Result<Self, Self::Error> {
        let scaled = value
            .checked_mul(Decimal::from(Self::MULTIPLIER))
            .ok_or_else(|| ValidationError::Quantity {
                reason: "value is too large to store".into(),
            })?;

        if !scaled.fract().is_zero() {
            return Err(ValidationError::Quantity {
                reason: "value has more than four decimal places".into(),
            });
        }

        scaled.to_i64().ok_or_else(|| ValidationError::Quantity {
            reason: "value is outside SQLite BIGINT range".into(),
        })?;

        let mut value = value;
        value.rescale(Self::SCALE);
        Ok(Self(value))
    }
}

impl From<Quantity> for Decimal {
    fn from(value: Quantity) -> Self {
        value.0
    }
}

impl FromStr for Quantity {
    type Err = ValidationError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let value = Decimal::from_str(value).map_err(|error| ValidationError::Quantity {
            reason: error.to_string(),
        })?;
        Self::try_from(value)
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Add for Quantity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("quantity addition overflow")
    }
}

impl Sub for Quantity {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
            .expect("quantity subtraction overflow")
    }
}

impl Mul<Decimal> for Quantity {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        self.checked_mul(rhs)
            .expect("quantity multiplication produced an invalid quantity")
    }
}

impl Mul<Price> for Quantity {
    type Output = Price;

    fn mul(self, rhs: Price) -> Self::Output {
        self.checked_mul_price(rhs)
            .expect("quantity and price multiplication produced an invalid price")
    }
}

impl Div<Decimal> for Quantity {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        self.checked_div(rhs)
            .expect("quantity division produced an invalid quantity")
    }
}

impl Div for Quantity {
    type Output = Decimal;

    fn div(self, rhs: Self) -> Self::Output {
        self.checked_ratio(rhs).expect("quantity division failed")
    }
}

impl AddAssign for Quantity {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Quantity {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl FromSql<BigInt, Sqlite> for Quantity {
    fn from_sql(value: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
        let value = i64::from_sql(value)?;
        Ok(Self(Decimal::from_i128_with_scale(
            i128::from(value),
            Self::SCALE,
        )))
    }
}

impl ToSql<BigInt, Sqlite> for Quantity {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let value = self
            .as_stored_integer()
            .ok_or_else(|| ValidationError::Quantity {
                reason: "value is outside SQLite BIGINT range".into(),
            })?;
        out.set_value(value);
        Ok(IsNull::No)
    }
}

impl Default for Quantity {
    fn default() -> Self {
        Self(Decimal::default())
    }
}

impl Default for Price {
    fn default() -> Self {
        Self(Decimal::default())
    }
}
