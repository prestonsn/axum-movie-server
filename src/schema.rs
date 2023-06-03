// @generated automatically by Diesel CLI.

diesel::table! {
    movies (id) {
        id -> Int4,
        title -> Varchar,
        year -> Int4,
        description -> Text,
    }
}
