use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Insertable, Deserialize, Serialize, Debug)]
#[diesel(table_name = crate::schema::requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Request {
    pub id: i32,
    pub status: String,
    pub user_id: String,
    pub mode: String,
    pub ip_address: String,
    pub port: i32,
    pub reward_address: String,
}

#[derive(Selectable, Insertable, Deserialize, Serialize)]
#[diesel(table_name = crate::schema::requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RequestNewItem {
    pub user_id: String,
    pub mode: String,
    pub ip_address: String,
    pub port: i32,
    pub reward_address: String,
}
