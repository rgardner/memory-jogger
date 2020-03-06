//! Database Schema definitions.

table! {
    saved_items (id) {
        id -> Int4,
        pocket_id -> Varchar,
        title -> Varchar,
        body -> Text,
    }
}
