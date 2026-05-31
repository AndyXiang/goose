use crate::{
    core::{Event, Strategy, SystemEvent},
    data::{Bar, DataBase, DataHandler, Entity, TimeStamp},
    error::{Error, Result},
    backtest::Broker,
};
use std::collections::BTreeMap;
use uuid::Uuid;


// backtest engine
pub struct BTEngine<'a, S: Strategy, B: Broker> {
    data: DataHandler<'a>,
    events: Vec<Event>,
    strat: S,
    broker: B,
    // risk: ,
}

impl<'a, S: Strategy, B: Broker> BTEngine<'a, S, B> {
    pub fn new(db: &'a DataBase, strat: S, broker: B) -> Self {
        Self {
            data: DataHandler::new(db),
            events: vec![Event::System(SystemEvent::BacktestInit)],
            strat,
            broker
        }
    }

    pub fn run(&mut self, start: TimeStamp, end: TimeStamp) -> Result<()> {
        todo!()
    }
}
