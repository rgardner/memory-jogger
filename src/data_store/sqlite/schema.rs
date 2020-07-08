table! {
    saved_items (id) {
        id -> Integer,
        user_id -> Integer,
        pocket_id -> Text,
        title -> Text,
        excerpt -> Nullable<Text>,
        url -> Nullable<Text>,
        time_added -> Nullable<Timestamp>,
    }
}

table! {
    users (id) {
        id -> Integer,
        email -> Text,
        pocket_access_token -> Nullable<Text>,
        last_pocket_sync_time -> Nullable<BigInt>,
    }
}

joinable!(saved_items -> users (user_id));

allow_tables_to_appear_in_same_query!(
    saved_items,
    users,
);
