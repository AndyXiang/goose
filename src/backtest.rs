use crate::{
    core::{Event, Strategy, SystemEvent},
    data::{Bar, DataBase, DataHandler, Entity, TimeStamp},
    error::{Error, Result},
};
use std::collections::BTreeMap;
use uuid::Uuid;

// backtest engine
pub struct BTEngine<'a, S: Strategy> {
    data: DataHandler<'a>,
    events: Vec<Event>,
    strat: S,
    // portfolio: ,
    // risk: ,
}

impl<'a, S: Strategy> BTEngine<'a, S> {
    pub fn new(db: &'a DataBase, strat: S) -> Self {
        Self {
            data: DataHandler::new(db),
            events: vec![Event::System(SystemEvent::BacktestInit)],
            strat,
        }
    }

    pub fn run(&mut self, start: TimeStamp, end: TimeStamp) -> Result<()> {
        todo!()
    }
}
