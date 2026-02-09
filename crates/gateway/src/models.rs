use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Insertable, Deserialize, Serialize, Debug)]
#[diesel(table_name = crate::schema::requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Request {
    pub id: i32,
    pub route: String,
    pub mode: String,
    pub ip_address: String,
    pub port: i32,
    pub reward_address: String,
}

#[derive(Selectable, Insertable, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RequestNewItem {
    pub route: String,
    pub user_id: i32,
    pub mode: String,
    pub ip_address: String,
    pub port: i32,
    pub reward_address: String,
    pub asset_name: Option<String>,
}

#[derive(Selectable, Insertable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub email: String,
    pub secret: String,
}
