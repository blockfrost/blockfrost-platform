use crate::errors::APIError;
use crate::{
    models::{Request, RequestNewItem},
    schema,
};
use deadpool_diesel::postgres::{Manager, Pool};
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(Clone)]
pub struct DB {
    pool: Pool,
}

impl DB {
    pub async fn new(database_url: &str) -> Self {
        let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
        let pool = Pool::builder(manager).build().expect("Failed to create pool.");
        let conn = pool.get().await.expect("Failed to get a connection.");

        conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .expect("Failed to run migrations.")
            .expect("Migration execution error.");

        Self { pool }
    }

    pub async fn insert_request(&self, request: RequestNewItem) -> Result<Request, APIError> {
        let db_pool = self.pool.get().await.map_err(|e| APIError::Unexpected(e.to_string()))?;

        let result = db_pool
            .interact(|db_pool| {
                diesel::insert_into(schema::requests::table)
                    .values(request)
                    .returning(Request::as_returning())
                    .get_result(db_pool)
            })
            .await??;

        Ok(result)
    }

    pub async fn authorize_user(&self, secret: String) -> Result<(), APIError> {
        let db_pool = self.pool.get().await?;

        let secret_clone = secret.clone();

        let result: bool = db_pool
            .interact(move |db_pool| {
                diesel::select(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "SELECT EXISTS (SELECT 1 FROM users WHERE secret = '{}')",
                    secret_clone
                )))
                .get_result(db_pool)
            })
            .await??;

        if !result {
            return Err(APIError::Unauthorized());
        }

        Ok(())
    }
}
