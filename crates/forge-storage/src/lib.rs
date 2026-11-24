pub mod config;
pub mod rbac;
pub mod repos;
pub mod runs;

use config::{DatabaseBackend, database_url_from_env};
use forge_core::ForgeResult;
use sqlx::{AnyPool, any::AnyPoolOptions};
use std::time::Duration;
use tracing::info;

pub use config::cloud_mode;
pub use rbac::{Permission, Role};
pub use runs::{MemoryStore, RunStore};

#[derive(Clone)]
pub struct ForgeStorage {
    pool: AnyPool,
    backend: DatabaseBackend,
}

impl ForgeStorage {
    pub async fn connect_from_env() -> ForgeResult<Self> {
        let url = database_url_from_env();
        Self::connect(&url).await
    }

    pub async fn connect(database_url: &str) -> ForgeResult<Self> {
        let backend = DatabaseBackend::detect(database_url);
        if backend == DatabaseBackend::Postgres && !cfg!(feature = "postgres") {
            return Err(forge_core::ForgeError::Storage(
                "PostgreSQL URL configured, but forge-storage was built without the `postgres` feature".into(),
            ));
        }

        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(if backend == DatabaseBackend::Sqlite {
                5
            } else {
                10
            })
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;

        info!(?backend, "database pool ready");
        Ok(Self { pool, backend })
    }

    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    pub fn backend(&self) -> DatabaseBackend {
        self.backend
    }

    pub async fn ping(&self) -> ForgeResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| forge_core::ForgeError::Storage(e.to_string()))?;
        Ok(())
    }
}
