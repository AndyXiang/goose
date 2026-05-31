use crate::{
    core::{ClockEvent, Event, FillEvent, MarketEvent, OrderEvent, RiskEvent},
    data::TimeStamp,
    error::{Error, Result},
};
use rust_decimal::Decimal;
use uuid::Uuid;

pub trait Broker {
    fn on_event(&mut self, event: Event) -> Result<Vec<Event>>;
}

// A simple broker with T+0
pub struct T0Broker {
    date: TimeStamp,
    commision_rate: Decimal,
    slippage_rate: Decimal,
}

impl T0Broker {
    pub fn new(start: TimeStamp, commision_rate: Decimal, slippage_rate: Decimal) -> Result<Self> {
        if commision_rate < Decimal::ZERO {
            return Err(Error::data("commision_rate must be non-negative"));
        }

        if commision_rate >= Decimal::ONE {
            return Err(Error::data("commision_rate must be less than 1"));
        }

        if slippage_rate < Decimal::ZERO {
            return Err(Error::data("slippage_rate must be non-negative"));
        }

        if slippage_rate >= Decimal::ONE {
            return Err(Error::data("slippage_rate must be less than 1"));
        }

        Ok(Self {
            date: start,
            commision_rate,
            slippage_rate,
        })
    }

    fn commision(&self, price: Decimal, amount: Decimal) -> Result<Decimal> {
        if let Some(x) = Decimal::checked_mul(price, amount) {
            if let Some(x) = Decimal::checked_mul(self.commision_rate, x) {
                return Ok(x);
            } else {
                return Err(Error::data("multiplicaion overflow"));
            }
        } else {
            return Err(Error::data("multiplicaion overflow"));
        }
    }

    fn slippage(&self, is_sell: bool, price: Decimal) -> Result<Decimal> {
        let r: Decimal = if is_sell {
            match Decimal::checked_add(Decimal::new(1, 0), self.slippage_rate) {
                Some(x) => x,
                None => {
                    return Err(Error::data("add overflow"));
                }
            }
        } else {
            match Decimal::checked_sub(Decimal::new(1, 0), self.slippage_rate) {
                Some(x) => x,
                None => {
                    return Err(Error::data("sub underflow"));
                }
            }
        };
        if let Some(x) = Decimal::checked_mul(price, r) {
            Ok(x)
        } else {
            Err(Error::data("multiplication overflow"))
        }
    }
}

impl Broker for T0Broker {
    fn on_event(&mut self, event: Event) -> Result<Vec<Event>> {
        match event {
            Event::Clock(ClockEvent::Open(ts)) => {
                self.date = ts;
            }
            Event::Clock(ClockEvent::Close(_)) => (), // do nothing at close
            Event::Market(MarketEvent::Bar { id, ts, bar }) => {
                todo!();
            }
            Event::Order(order) => match order {
                OrderEvent::Sell {
                    id,
                    ts,
                    amount,
                    price,
                } => {
                    let price = self.slippage(true, price)?;
                    let commision: Decimal = self.commision(price, amount)?;
                    return Ok(vec![Event::Fill(FillEvent::Sell {
                        order,
                        ts: self.date,
                        amount,
                        price,
                        commision,
                    })]);
                }
                OrderEvent::Buy {
                    id,
                    ts,
                    amount,
                    price,
                } => {
                    let price = self.slippage(false, price)?;
                    let commision: Decimal = self.commision(price, amount)?;
                    return Ok(vec![Event::Fill(FillEvent::Sell {
                        order,
                        ts: self.date,
                        amount,
                        price,
                        commision,
                    })]);
                }
            },
            // Position(PositionEvent),
            // Fill(FillEvent),
            // Risk(RiskEvent),
            // System(SystemEvent),
            _ => {
                todo!();
            }
        }
        Ok(vec![])
    }
}
