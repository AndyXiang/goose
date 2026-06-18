// @generated automatically by Diesel CLI.

diesel::table! {
    calendar (date) {
        date -> Text,
        is_open -> Bool,
    }
}

diesel::table! {
    daily_bars (id) {
        id -> Integer,
        symbol -> Text,
        date -> Text,
        price_adjust -> Text,
        open -> Nullable<BigInt>,
        high -> Nullable<BigInt>,
        low -> Nullable<BigInt>,
        close -> Nullable<BigInt>,
        volume -> Nullable<BigInt>,
        amount -> Nullable<BigInt>,
    }
}

diesel::joinable!(daily_bars -> calendar (date));

diesel::allow_tables_to_appear_in_same_query!(calendar, daily_bars,);
