// @generated automatically by Diesel CLI.

diesel::table! {
    requests (id) {
        id -> Int4,
        created_at -> Timestamp,
        #[max_length = 255]
        route -> Varchar,
        #[max_length = 255]
        mode -> Varchar,
        #[max_length = 45]
        ip_address -> Varchar,
        port -> Int4,
        #[max_length = 255]
        reward_address -> Varchar,
        user_id -> Int4,
        asset_name -> Nullable<Text>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        created_at -> Timestamp,
        user_id -> Int4,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        secret -> Varchar,
    }
}

diesel::joinable!(requests -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(requests, users,);
