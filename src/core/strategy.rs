use crate::{core::Event, error::Result};

pub trait Strategy {
    fn on_event(&mut self, event: &Event) -> Result<Vec<Event>>;
}
