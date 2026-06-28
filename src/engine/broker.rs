use super::{Order, Account, Event, MarketEvent};
// broker cannot fill an order higher than high or lower than low
// thus broker need to see the full bar even before close
// need sepcific implementation

pub trait Broker {
    fn on_order(&mut self, order: &Order, account: &mut Account) -> Event;
    fn preview(&mut self, market: &MarketEvent);
}



