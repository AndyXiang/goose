// @generated automatically by Diesel CLI.

diesel::table! {
    calendar (date) {
        date -> Text,
        market -> Text,
        is_open -> Integer,
    }
}

diesel::table! {
    daily_bars (id) {
        id -> Integer,
        symbol -> Integer,
        date -> Text,
        open -> Nullable<Integer>,
        high -> Nullable<Integer>,
        low -> Nullable<Integer>,
        close -> Nullable<Integer>,
        is_adjust -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(calendar, daily_bars,);
