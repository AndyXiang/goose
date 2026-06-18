#![allow(unused)]
use crate::{
    data::{DataBase, Date, DateBar, Price, PriceAdjust},
    error::{BacktestError, Result},
    strat::Strategy,
};
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};

pub enum Event {
    Market(MarketEvent),
    Order(OrderEvent),
    Fill(FillEvent),
    System(SystemEvent),
}

// cross-section over the market
pub enum MarketEvent {
    // only open price is seen when market opens
    Open(HashMap<String, Option<Price>>),
    // see full bar when market closes
    Close(HashMap<String, DateBar>),
}

pub enum OrderEvent {
    Buy {
        symbol: String,
        price: Price,
        quantity: u32,
    },
    Sell {
        symbol: String,
        price: Price,
        quantity: u32,
    },
}

pub enum FillEvent {
    Succeed {
        symbol: String,
        price: Price,
        quantity: i32,
        comm: Price,
    },
    Reject,
}

pub enum SystemEvent {
    Init,
    End,
}

pub struct Account {
    cash: Price,
    position: HashMap<String, i32>,
}

impl Account {
    pub fn new(cash: Price) -> Self {
        Self {
            cash,
            position: HashMap::new(),
        }
    }

    pub fn is_affordable(&self, order: &OrderEvent) -> bool {
        todo!()
    }

    pub const fn cash(&self) -> Price {
        self.cash
    }

    pub fn position(&self, symbol: &str) -> i32 {
        self.position.get(symbol).copied().unwrap_or_default()
    }

    pub fn change_position(&mut self, symbol: String, quantity: i32) -> Result<()> {
        if let Some(v) = self.position.get_mut(&symbol) {
            if let Some(q) = v.checked_add(quantity) {
                *v = q;
            } else {
                return Err(BacktestError::PositionOverflow { symbol }.into());
            }
        } else {
            self.position.insert(symbol, quantity);
        }
        Ok(())
    }

    /// Applies a fill atomically and returns whether it was affordable.
    pub fn apply_fill(
        &mut self,
        symbol: String,
        price: Price,
        quantity: i32,
        commission: Price,
    ) -> Result<bool> {
        let trade_value = price
            .checked_mul(Decimal::from(quantity))
            .ok_or(BacktestError::CashOverflow)?;
        let cash_after_fill = self
            .cash
            .checked_sub(trade_value)
            .and_then(|cash| cash.checked_sub(commission))
            .ok_or(BacktestError::CashOverflow)?;

        if Decimal::from(cash_after_fill).is_sign_negative() {
            return Ok(false);
        }

        self.change_position(symbol, quantity)?;
        self.cash = cash_after_fill;
        Ok(true)
    }
}

pub struct BTLogger {}

impl BTLogger {
    pub fn new(config: &BTConfig) -> Self {
        todo!();
        Self {}
    }
}

pub struct BTConfig {
    pub price_adjust: PriceAdjust,
}

pub struct BTEngine<B, S>
where
    B: Broker,
    S: Strategy,
{
    account: Account,
    broker: B,
    events: VecDeque<Event>,
    strat: S,
    log: BTLogger,
    config: BTConfig,
}

impl<B, S> BTEngine<B, S>
where
    B: Broker,
    S: Strategy,
{
    pub fn new(cash: Price, broker: B, strat: S, config: BTConfig) -> Self {
        Self {
            account: Account::new(cash),
            broker,
            events: VecDeque::from([
                Event::System(SystemEvent::Init),
                Event::System(SystemEvent::End),
            ]),
            strat,
            log: BTLogger::new(&config),
            config,
        }
    }

    pub fn run(&mut self, start: Date, end: Date, database: &mut DataBase) -> Result<()> {
        assert!(start <= end, "start date {start} is after end date {end}");

        let trading_days = database.get_trading_days(&start, &end)?;
        if trading_days.is_empty() {
            // no trading day, end back test
            return Ok(());
        }

        while let Some(event) = self.events.pop_front() {
            use Event::*;
            match event {
                System(system_event) => {
                    use SystemEvent::*;
                    match system_event {
                        Init => {
                            use MarketEvent::*;
                            // get all data and push
                            for trade_date in &trading_days {
                                let cross_section = database
                                    .get_cross_section(trade_date, self.config.price_adjust)?;
                                self.events.push_front(Market(Close(cross_section.clone())));
                                self.events.push_front(Market(Open(
                                    cross_section
                                        .into_iter()
                                        .map(|(symbol, bar)| (symbol, bar.ohlc.open))
                                        .collect(),
                                )));
                            }
                        }
                        End => {
                            // end back test and compute results
                            todo!();
                        }
                    }
                }
                Market(market_event) => {
                    // emit market event to strategy
                    self.events
                        .push_front(self.strat.on_market(&market_event, &self.account));
                }
                Order(order_event) => {
                    // emit order to broker
                    self.events
                        .push_front(self.broker.on_order(&order_event, &mut self.account));
                }
                Fill(fill_event) => {
                    // change account if filled
                    // maybe emit fill to strategy
                    use FillEvent::*;
                    match fill_event {
                        Succeed {
                            symbol,
                            price,
                            quantity,
                            comm,
                        } => {
                            self.account.apply_fill(symbol, price, quantity, comm)?;
                        }
                        Reject => (),
                    }
                }
                _ => {
                    todo!();
                }
            }
        }

        todo!()
    }
}

// broker cannot fill an order higher than high or lower than low
// thus broker need to see the full bar even before close
// need sepcific implementation

pub trait Broker {
    fn on_order(&mut self, order_event: &OrderEvent, account: &mut Account) -> Event;
}
