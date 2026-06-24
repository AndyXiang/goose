use super::*;
use crate::data::{Date, Price, Quantity};
use std::collections::{BTreeMap, HashMap};

pub trait Logger {
    fn log(&mut self, event: &Event);
    // fn finish(&self);
}

#[derive(Debug, Default)]
pub struct EquityLogger {
    equity_curve: BTreeMap<Date, EquityPoint>,
    cash: Price,
    position: HashMap<String, Quantity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquityPoint {
    pub cash: Price,
    pub position: HashMap<String, Quantity>,
    pub equity: Price,
}

impl EquityLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn equity_curve(&self) -> &BTreeMap<Date, EquityPoint> {
        &self.equity_curve
    }
}

impl Logger for EquityLogger {
    fn log(&mut self, event: &Event) {
        use Event::*;
        match event {
            Market(market) => {
                use MarketEvent::*;
                match market {
                    Open(_) => {}
                    Close(cross_section) => {
                        if let Some(date) = cross_section.values().next().map(|bar| bar.date) {
                            let market_value = cross_section.iter().fold(
                                Price::default(),
                                |total, (symbol, bar)| {
                                    let position =
                                        self.position.get(symbol).copied().unwrap_or_default();
                                    match bar.ohlc.close {
                                        Some(close) => total + close * position,
                                        None => total,
                                    }
                                },
                            );
                            self.equity_curve.insert(
                                date,
                                EquityPoint {
                                    cash: self.cash,
                                    position: self.position.clone(),
                                    equity: self.cash + market_value,
                                },
                            );
                        }
                    }
                }
            }
            Fill(fill) => {
                if let FillEvent::Succeed(fills) = fill {
                    for (symbol, (price, quantity, commission)) in fills {
                        let trade_value = *price * *quantity;
                        self.cash -= trade_value;
                        self.cash -= *commission;
                        *self.position.entry(symbol.clone()).or_default() += *quantity;
                    }
                }
            }
            Order(_) | System(_) => {}
        }
    }
}

impl Default for EquityPoint {
    fn default() -> Self {
        Self {
            cash: Price::default(),
            position: HashMap::new(),
            equity: Price::default(),
        }
    }
}
