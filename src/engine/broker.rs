use crate::{
    data::{Price, Quantity},
    engine::{Account, Event, FillEvent, MarketEvent, Order},
};
use std::collections::HashMap;
// broker cannot fill an order higher than high or lower than low
// thus broker need to see the full bar even before close
// need sepcific implementation

pub trait Broker {
    fn on_order(&mut self, order: &Order, account: &mut Account) -> Event;
    fn preview(&mut self, market: &MarketEvent);
}

pub struct OpenPriceBroker {
    section_open: HashMap<String, Price>,
    commission_rate: Quantity,
    allow_short: bool,
    allow_margin: bool,
}

impl OpenPriceBroker {
    pub fn new(commission_rate: Quantity, allow_short: bool, allow_margin: bool) -> Self {
        Self {
            section_open: HashMap::new(),
            commission_rate,
            allow_short,
            allow_margin,
        }
    }
}

impl Broker for OpenPriceBroker {
    fn on_order(&mut self, order: &Order, account: &mut Account) -> Event {
        let fills: HashMap<_, _> = order
            .order
            .iter()
            .filter_map(|(symbol, (_limit_price, quantity))| {
                let fill_price = *self.section_open.get(symbol)?;

                if !self.allow_short {
                    let next_position = account.position(symbol).checked_add(*quantity)?;
                    if next_position.is_negative() {
                        return None;
                    }
                }

                let commission = fill_price * self.commission_rate;
                if !self.allow_margin {
                    let cash_after_fill = account
                        .cash()
                        .checked_sub(fill_price * *quantity)
                        .and_then(|cash| cash.checked_sub(commission))?;
                    if cash_after_fill.is_negative() {
                        return None;
                    }
                }

                Some((symbol.clone(), (fill_price, *quantity, commission)))
            })
            .collect();

        if fills.is_empty() {
            Event::Fill(FillEvent::Reject)
        } else {
            Event::Fill(FillEvent::Succeed(fills))
        }
    }

    fn preview(&mut self, market: &MarketEvent) {
        use MarketEvent::*;
        if let Preview(section) = market {
            self.section_open = section
                .iter()
                .map(|(symbol, bar)| (symbol.clone(), bar.ohlc.open))
                .collect();
        }
    }
}
