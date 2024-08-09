// @generated automatically by Diesel CLI.

diesel::table! {
    requests (id) {
        id -> Int4,
        created_at -> Timestamp,
        #[max_length = 255]
        user_id -> Varchar,
        #[max_length = 255]
        status -> Varchar,
        #[max_length = 255]
        mode -> Varchar,
        #[max_length = 45]
        ip_address -> Varchar,
        port -> Int4,
        #[max_length = 255]
        reward_address -> Varchar,
    }
}
