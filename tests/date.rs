use chrono::NaiveDate;
use goose::data::Date;
use std::collections::HashSet;

#[test]
fn date_parses_and_formats_as_iso_8601() {
    let date: Date = "2026-06-13".parse().unwrap();

    assert_eq!(date.to_string(), "2026-06-13");
}

#[test]
fn date_rejects_invalid_values() {
    assert!("2026-02-29".parse::<Date>().is_err());
    assert!(Date::from_ymd(2026, 13, 1).is_err());
}

#[test]
fn date_converts_to_and_from_naive_date() {
    let naive = NaiveDate::from_ymd_opt(2026, 6, 13).unwrap();
    let date = Date::from(naive);

    assert_eq!(date.as_naive_date(), &naive);
    assert_eq!(date.into_naive_date(), naive);
    assert_eq!(NaiveDate::from(date), naive);
}

#[test]
fn date_supports_ordering_and_hashing() {
    let earlier = Date::from_ymd(2026, 6, 12).unwrap();
    let later = Date::from_ymd(2026, 6, 13).unwrap();
    let dates = HashSet::from([earlier, later, earlier]);

    assert!(earlier < later);
    assert_eq!(dates.len(), 2);
}
