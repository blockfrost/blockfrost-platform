use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[cfg(not(target_os = "windows"))]
use diesel::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
#[cfg_attr(not(target_os = "windows"), derive(Queryable, Selectable, Insertable))]
#[cfg_attr(not(target_os = "windows"), diesel(table_name = crate::schema::requests))]
#[cfg_attr(not(target_os = "windows"), diesel(check_for_backend(diesel::pg::Pg)))]
pub struct Request {
    pub id: i32,
    pub route: String,
    pub mode: String,
    pub ip_address: String,
    pub port: i32,
    pub reward_address: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[cfg_attr(not(target_os = "windows"), derive(Selectable, Insertable))]
#[cfg_attr(not(target_os = "windows"), diesel(table_name = crate::schema::requests))]
#[cfg_attr(not(target_os = "windows"), diesel(check_for_backend(diesel::pg::Pg)))]
pub struct RequestNewItem {
    pub route: String,
    pub user_id: i32,
    pub mode: String,
    pub ip_address: String,
    pub port: i32,
    pub reward_address: String,
    pub asset_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[cfg_attr(not(target_os = "windows"), derive(Selectable, Insertable, Queryable))]
#[cfg_attr(not(target_os = "windows"), diesel(table_name = crate::schema::users))]
#[cfg_attr(not(target_os = "windows"), diesel(check_for_backend(diesel::pg::Pg)))]
pub struct User {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub user_id: i32,
    pub email: String,
    pub secret: String,
}
