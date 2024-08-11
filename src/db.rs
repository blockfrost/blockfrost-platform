use deadpool_diesel::postgres::{Manager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

pub async fn init_db(database_url: &str) -> Pool {
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager)
        .build()
        .expect("Failed to create pool.");

    let conn = pool.get().await.expect("Failed to get a connection.");

    conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
        .await
        .expect("Failed to run migrations.")
        .expect("Migration execution error.");

    pool
}
