use crate::errors::APIError;
use crate::{
    models::{Request, RequestNewItem, User},
    schema,
};
use deadpool_diesel::postgres::{Manager, Pool};
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use schema::users::dsl::*;
use std::num::NonZeroUsize;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(Clone)]
pub struct DB {
    pool: Pool,
}

/// A point-in-time snapshot of the connection pool state, for metrics.
pub struct PoolStatus {
    pub max_size: usize,
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
}

impl DB {
    pub async fn new(database_url: &str, pool_max_size: NonZeroUsize) -> Self {
        let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
        let pool = Pool::builder(manager)
            .max_size(pool_max_size.get())
            .build()
            .expect("Failed to create pool.");

        if cfg!(feature = "dev_mock_db") {
            return Self { pool };
        }

        let connection = pool.get().await.expect("Failed to get a connection.");
        connection
            .interact(|c| c.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .expect("Failed to run migrations.")
            .expect("Migration execution error.");

        Self { pool }
    }

    /// Verifies database connectivity by running a trivial query through the connection pool
    pub async fn ping(&self, timeout: std::time::Duration) -> Result<(), String> {
        if cfg!(feature = "dev_mock_db") {
            return Ok(());
        }

        let check = async {
            let connection = self
                .pool
                .get()
                .await
                .map_err(|e| format!("database pool error: {e}"))?;
            connection
                .interact(|c| diesel::sql_query("SELECT 1").execute(c))
                .await
                .map_err(|e| format!("database interaction error: {e}"))?
                .map_err(|e| format!("database query error: {e}"))?;
            Ok(())
        };

        tokio::time::timeout(timeout, check)
            .await
            .unwrap_or_else(|_| {
                Err(format!(
                    "database health check timed out after {}s (pool exhausted or database unreachable)",
                    timeout.as_secs()
                ))
            })
    }

    pub fn pool_status(&self) -> PoolStatus {
        let status = self.pool.status();
        PoolStatus {
            max_size: status.max_size,
            size: status.size,
            available: status.available,
            waiting: status.waiting,
        }
    }

    pub async fn insert_request(&self, request: RequestNewItem) -> Result<Request, APIError> {
        if cfg!(feature = "dev_mock_db") {
            return Ok(Request {
                id: 42,
                route: request.route,
                mode: request.mode,
                ip_address: request.ip_address,
                port: request.port,
                reward_address: request.reward_address,
            });
        }

        let db_pool = self.pool.get().await?;

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

    pub async fn authorize_user(&self, secret_param: String) -> Result<User, APIError> {
        if cfg!(feature = "dev_mock_db") {
            return Ok(User {
                id: 31337,
                created_at: chrono::NaiveDateTime::parse_from_str(
                    "2015-09-05 23:56:04",
                    "%Y-%m-%d %H:%M:%S",
                )
                .unwrap(),
                user_id: 31337,
                email: "xxx@xxx.xxx".to_string(),
                secret: "xxxxxxxx".to_string(),
            });
        }

        let db_pool = self.pool.get().await?;

        let user_result: Option<User> = db_pool
            .interact(|db_pool| {
                users
                    .filter(secret.eq(secret_param))
                    .first::<User>(db_pool)
                    .optional()
            })
            .await??;

        if let Some(user) = user_result {
            Ok(user)
        } else {
            Err(APIError::Unauthorized())
        }
    }
}
