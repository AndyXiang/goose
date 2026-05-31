use crate::{
    core::{Event, FillEvent, MarketEvent, OrderEvent},
    data::Entity,
    error::Result,
};
use uuid::Uuid;

pub trait Portfolio {
    fn asset(&self) -> Vec<Uuid>;
    fn select_asset(&mut self, market: MarketEvent) -> Result<Vec<Event>>;
}

// a strategy should include:
// 1. choose asset -> Portfolio
// 2. sell, buy -> order and fill
// 3. handle risk

pub trait Strategy: Portfolio {
    fn create_order(&mut self, market: MarketEvent) -> Result<Vec<OrderEvent>>;
    fn on_fill(&mut self, fill: FillEvent) -> Result<()>;
}

pub mod collections {
    use super::*;
    use crate::data::StockCN;

    // cn stock as singlet portfolio
    impl Portfolio for StockCN {
        fn asset(&self) -> Vec<Uuid> {
            vec![self.id]
        }
        fn select_asset(&mut self, market: MarketEvent) -> Result<Vec<Event>> {
            Ok(Vec::new())
        }
    }

    #[derive(Debug, Clone)]
    pub struct StockCNSet {
        stocks: Vec<StockCN>,
    }

    // a static stock portfolio
    impl Portfolio for StockCNSet {
        fn asset(&self) -> Vec<Uuid> {
            self.stocks.iter().map(|s| s.id).collect()
        }
        fn select_asset(&mut self, market: MarketEvent) -> Result<Vec<Event>> {
            Ok(Vec::new())
        }
    }
}
