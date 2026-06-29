use goose::{
    data::{CalendarEntry, DataBase, Date, DateBar, Ohlc, Price, Quantity},
    engine::{Account, BTEngine, Broker, Event, FillEvent, Logger, MarketEvent, Order},
    strat::Strategy,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

fn price(value: &str) -> Price {
    value.parse().unwrap()
}

fn quantity(value: &str) -> Quantity {
    value.parse().unwrap()
}

fn date(value: &str) -> Date {
    value.parse().unwrap()
}

fn bar(symbol: &str, date: &str, open: &str, close: &str) -> DateBar {
    let volume = quantity("1000");
    DateBar::new(
        symbol,
        date.parse().unwrap(),
        Ohlc::new(
            price(open),
            price(open).max(price(close)),
            price(open).min(price(close)),
            price(close),
        )
        .unwrap(),
        volume,
        price(close) * volume,
    )
    .unwrap()
}

#[test]
fn apply_fill_updates_cash_and_position_for_buys_and_sells() {
    let mut account = Account::new(price("100.0000"));

    account
        .apply_fill(
            "AAPL".into(),
            price("10.0000"),
            quantity("5.0000"),
            price("1.0000"),
        )
        .unwrap();
    assert_eq!(account.cash(), price("49.0000"));
    assert_eq!(account.position("AAPL"), quantity("5.0000"));

    account
        .apply_fill(
            "AAPL".into(),
            price("10.0000"),
            quantity("-2.0000"),
            price("1.0000"),
        )
        .unwrap();
    assert_eq!(account.cash(), price("68.0000"));
    assert_eq!(account.position("AAPL"), quantity("3.0000"));
}

#[test]
fn apply_fill_allows_negative_cash_for_broker_approved_fills() {
    let mut account = Account::new(price("100.0000"));

    account
        .apply_fill(
            "AAPL".into(),
            price("20.0000"),
            quantity("5.0000"),
            price("1.0000"),
        )
        .unwrap();

    assert_eq!(account.cash(), price("-1.0000"));
    assert_eq!(account.position("AAPL"), quantity("5.0000"));
}

struct BuyOnce {
    bought: bool,
}

impl Strategy for BuyOnce {
    fn on_market(&mut self, market_event: &MarketEvent, _account: &Account) -> Order {
        if self.bought {
            return Order {
                order: HashMap::new(),
            };
        }

        let MarketEvent::Open(open) = market_event else {
            return Order {
                order: HashMap::new(),
            };
        };

        let Some((_date, Some(open_price))) = open.get("AAPL") else {
            return Order {
                order: HashMap::new(),
            };
        };

        self.bought = true;
        Order {
            order: HashMap::from([("AAPL".into(), (*open_price, quantity("1.0000")))]),
        }
    }
}

struct FillAllBroker;

impl Broker for FillAllBroker {
    fn on_order(&mut self, order: &Order, _account: &mut Account) -> Event {
        Event::Fill(FillEvent::Succeed(
            order
                .order
                .iter()
                .map(|(symbol, (fill_price, quantity))| {
                    (symbol.clone(), (*fill_price, *quantity, price("0.0000")))
                })
                .collect(),
        ))
    }

    fn preview(&mut self, _market: &MarketEvent) {}
}

#[derive(Clone, Default)]
struct EventLog(Rc<RefCell<Vec<Event>>>);

impl Logger for EventLog {
    fn log(&mut self, event: &Event) {
        self.0.borrow_mut().push(event.clone());
    }
}

#[test]
fn engine_runs_a_minimal_open_fill_close_backtest_pipeline() {
    let mut database = DataBase::new(":memory:");
    database
        .insert_calendar(&[
            CalendarEntry {
                date: date("2026-06-12"),
                is_open: true,
            },
            CalendarEntry {
                date: date("2026-06-15"),
                is_open: true,
            },
        ])
        .unwrap();
    database
        .insert_bars(&[
            bar("AAPL", "2026-06-12", "10.0000", "12.0000"),
            bar("AAPL", "2026-06-15", "12.0000", "13.0000"),
        ])
        .unwrap();

    let event_log = EventLog::default();
    let events = event_log.0.clone();
    let mut engine = BTEngine::new(
        price("100.0000"),
        FillAllBroker,
        BuyOnce { bought: false },
        event_log,
    );

    engine
        .run(date("2026-06-12"), date("2026-06-15"), &mut database)
        .unwrap();

    let events = events.borrow();
    assert!(events.iter().any(|event| matches!(event, Event::Fill(_))));
    assert!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Market(MarketEvent::Close(_))))
            .count()
            == 2
    );
}
