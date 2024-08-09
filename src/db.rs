pub async fn init_db(database_url: String) -> deadpool_diesel::postgres::Pool {
    let manager =
        deadpool_diesel::postgres::Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);

    let pool = deadpool_diesel::postgres::Pool::builder(manager)
        .build()
        .unwrap();

    pool
}
