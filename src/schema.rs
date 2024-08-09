// @generated automatically by Diesel CLI.

diesel::table! {
    requests (id) {
        id -> Int4,
        #[max_length = 255]
        status -> Varchar,
    }
}
