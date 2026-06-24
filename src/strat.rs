use crate::{
    // data::{Date, Price},
    engine::{Account, MarketEvent, Order},
    // error::{Error, Result},
};

pub trait Strategy {
    fn on_market(&mut self, market_event: &MarketEvent, account: &Account) -> Order;
}
