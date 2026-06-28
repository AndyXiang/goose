use super::{Broker, Event, FillEvent, Logger, MarketEvent, Order, SystemEvent};
use crate::{
    data::{DataBase, Date, Price, Quantity},
    error::{BacktestError, Result},
    strat::Strategy,
};
use std::collections::{HashMap, VecDeque};

pub struct Account {
    cash: Price,
    position: HashMap<String, Quantity>,
}

impl Account {
    pub fn new(cash: Price) -> Self {
        Self {
            cash,
            position: HashMap::new(),
        }
    }

    // pub fn is_affordable(&self, order: &OrderEvent) -> bool {
    //     todo!()
    // }

    pub const fn cash(&self) -> Price {
        self.cash
    }

    pub fn position(&self, symbol: &str) -> Quantity {
        self.position.get(symbol).copied().unwrap_or_default()
    }

    pub fn change_position(&mut self, symbol: String, quantity: Quantity) -> Result<()> {
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

    /// Applies a broker-approved fill atomically.
    pub fn apply_fill(
        &mut self,
        symbol: String,
        price: Price,
        quantity: Quantity,
        commission: Price,
    ) -> Result<()> {
        let trade_value: Price = price * quantity;
        let cash_after_fill = self
            .cash
            .checked_sub(trade_value)
            .and_then(|cash| cash.checked_sub(commission))
            .ok_or(BacktestError::CashOverflow)?;

        self.change_position(symbol, quantity)?;
        self.cash = cash_after_fill;
        Ok(())
    }
}

pub struct BTEngine<B, S, L>
where
    B: Broker,
    S: Strategy,
    L: Logger,
{
    account: Account,
    broker: B,
    events: VecDeque<Event>,
    strat: S,
    logger: L,
}

impl<B, S, L> BTEngine<B, S, L>
where
    B: Broker,
    S: Strategy,
    L: Logger,
{
    pub fn new(cash: Price, broker: B, strat: S, logger: L) -> Self {
        Self {
            account: Account::new(cash),
            broker,
            events: VecDeque::from([
                Event::System(SystemEvent::Init),
                Event::System(SystemEvent::End),
            ]),
            strat,
            logger,
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
            use Event::{Fill, Market, System};
            self.logger.log(&event);
            match event {
                System(system_event) => {
                    use SystemEvent::*;
                    match system_event {
                        Init => {
                            use MarketEvent::*;
                            // get all data and push
                            for trade_date in trading_days.iter().rev() {
                                let cross_section = database.get_cross_section(trade_date)?;
                                let preview = Market(Preview(cross_section.clone()));
                                let open = Market(Open(
                                    cross_section
                                        .iter()
                                        .map(|(symbol, bar)| {
                                            (symbol.clone(), (*trade_date, bar.ohlc.open.clone()))
                                        })
                                        .collect(),
                                ));
                                let close = Market(Close(cross_section));
                                self.events.push_front(close);
                                self.events.push_front(open);
                                self.events.push_front(preview);
                            }
                        }
                        End => {
                            // end back test and compute results
                            break;
                        }
                    }
                }
                Market(market_event) => {
                    use MarketEvent::*;
                    match market_event {
                        Open(_) => {
                            // emit market event to strategy
                            let strat_action: Order =
                                self.strat.on_market(&market_event, &self.account);
                            self.events.push_front(Event::Order(strat_action));
                        }
                        Close(_) => {
                            // only accept orders when market is open
                            self.strat.on_market(&market_event, &self.account);
                        }
                        Preview(_) => {
                            self.broker.preview(&market_event);
                        }
                    }
                }
                Event::Order(order_event) => {
                    // emit order to broker
                    self.events
                        .push_front(self.broker.on_order(&order_event, &mut self.account));
                }
                Fill(fill_event) => {
                    // change account if filled
                    // maybe emit fill to strategy
                    use FillEvent::*;
                    match &fill_event {
                        Succeed(fills) => {
                            for (symbol, (price, quantity, comm)) in fills {
                                self.account.apply_fill(
                                    symbol.clone(),
                                    *price,
                                    *quantity,
                                    *comm,
                                )?;
                            }
                        }
                        Reject => (),
                    };
                }
            }
        }
        Ok(())
    }
}
