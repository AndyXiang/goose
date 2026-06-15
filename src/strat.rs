#![allow(unused)]
use crate::{
    data::{Date, Price},
    engine::{Account, Event, MarketEvent},
    error::{Error, Result},
};

pub trait Strategy {
    fn on_market(&mut self, market_event: &MarketEvent, account: &Account) -> Event;
}
