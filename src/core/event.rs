use crate::data::{Bar, Entity, TimeStamp};
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

pub enum OrderEvent {
    Sell {
        id: Uuid,
        ts: TimeStamp,
        amount: Decimal,
        price: Decimal,
    },
    Buy {
        id: Uuid,
        ts: TimeStamp,
        amount: Decimal,
        price: Decimal,
    },
}

pub enum FillEvent {
    Sell {
        order: OrderEvent,
        ts: TimeStamp,
        amount: Decimal,
        price: Decimal,
        commision: Decimal,
    },
    Buy {
        order: OrderEvent,
        ts: TimeStamp,
        amount: Decimal,
        price: Decimal,
        commision: Decimal,
    },
    Cancel { order: OrderEvent, ts: TimeStamp},
}

pub enum RiskEvent {}

pub enum SystemEvent {
    BacktestInit,
}
