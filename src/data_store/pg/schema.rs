table! {
    saved_items (id) {
        id -> Int4,
        user_id -> Int4,
        pocket_id -> Varchar,
        title -> Varchar,
        excerpt -> Nullable<Text>,
        url -> Nullable<Text>,
        time_added -> Nullable<Timestamp>,
    }
}

table! {
    users (id) {
        id -> Int4,
        email -> Varchar,
        pocket_access_token -> Nullable<Varchar>,
        last_pocket_sync_time -> Nullable<Int8>,
    }
}

joinable!(saved_items -> users (user_id));

allow_tables_to_appear_in_same_query!(
    saved_items,
    users,
);
