use crate::data::{Entity, Bar, TimeStamp};
use rust_decimal::Decimal;
use uuid::Uuid;

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
    Bar { id: Uuid, ts: TimeStamp, bar: Bar },
}

pub enum ClockEvent {
    Open(TimeStamp),
    Close(TimeStamp),
}

pub struct PositionEvent {
    id: Uuid,
    weight: Decimal,
}

pub struct OrderEvent {
    id: Uuid,
    name: String,
    code: String,
    ts: TimeStamp,
    amount: Decimal,
}

pub struct FillEvent {
    id: Uuid,
    name: String,
    code: String,
    ts: TimeStamp,
    amount: Decimal,
    price: Decimal,
    commission: Decimal,
}

pub enum RiskEvent {
    
}

pub enum SystemEvent {
    BacktestInit
}
