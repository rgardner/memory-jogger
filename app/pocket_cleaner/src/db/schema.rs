table! {
    saved_items (id) {
        id -> Int4,
        pocket_id -> Varchar,
        title -> Varchar,
        body -> Text,
    }
}

table! {
    users (id) {
        id -> Int4,
        pocket_user_token -> Varchar,
        email -> Varchar,
    }
}

allow_tables_to_appear_in_same_query!(
    saved_items,
    users,
);
