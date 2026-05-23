use crate::{core::Event, error::Result, data::Entity};
use uuid::Uuid;

pub trait Strategy {
    fn assets(&self) -> &[Uuid];
    fn on_event(&mut self, event: &Event) -> Result<Vec<Event>>;
}

pub mod collections {

}
