use crate::data::{Date, DateBar, Price, Quantity};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Event {
    Market(MarketEvent),
    // Time(TimeEvent),
    Order(Order),
    Fill(FillEvent),
    System(SystemEvent),
}

// cross-section over the market
#[derive(Debug, Clone)]
pub enum MarketEvent {
    // only open price is seen when market opens
    Open(HashMap<String, (Date, Option<Price>)>),
    // see full bar when market closes
    Close(HashMap<String, DateBar>),
    // preview all day data fro broker
    Preview(HashMap<String, DateBar>),
}

#[derive(Debug, Clone)]
pub enum TimeEvent {
    TradingDay(Date),
    NonTradingDay(Date),
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order: HashMap<String, (Price, Quantity)>,
}

/// key=symbol, value=(fill_price, quantity, commission)
#[derive(Debug, Clone)]
pub enum FillEvent {
    Succeed(HashMap<String, (Price, Quantity, Price)>),
    Reject,
}

#[derive(Debug, Clone)]
pub enum SystemEvent {
    Init,
    End,
}
