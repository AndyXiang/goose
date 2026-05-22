use crate::data::{Asset, AssetSymbol, Bar, TimeStamp};
use rust_decimal::Decimal;

pub enum Event {
    Market(MarketEvent), 
    Clock(ClockEvent),
    Position(PositionEvent),
    Order(OrderEvent),
    Fill(FillEvent),
    Risk(RiskEvent),
    System(SystemEvent),
}

pub enum MarketEvent {
    Bar { symbol: AssetSymbol, ts: TimeStamp, bar: Bar },
}

pub enum ClockEvent {
    Open(TimeStamp),
    Close(TimeStamp),
}

pub struct PositionEvent {
    symbol: AssetSymbol,
    weight: Decimal,
}

pub struct OrderEvent {
    symbol: AssetSymbol,
    name: String,
    code: String,
    ts: TimeStamp,
    amount: Decimal,
    reason: String,
}

pub struct FillEvent {
    symbol: AssetSymbol,
    name: String,
    code: String,
    ts: TimeStamp,
    amount: Decimal,
    price: Decimal,
    commission: Decimal,
}

pub enum RiskEvent {
    
}

pub enum SystemEvent {}
