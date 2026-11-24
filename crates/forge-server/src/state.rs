use forge_engine::{RunEngine, RunEngineConfig};
use forge_secrets::SecretsManager;
use forge_storage::ForgeStorage;
use forge_storage::MemoryStore;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<ForgeStorage>,
    pub cloud: bool,
    engines: Arc<Mutex<HashMap<String, Arc<RunEngine>>>>,
}

impl AppState {
    pub async fn bootstrap() -> Result<Self, forge_core::ForgeError> {
        let storage = Arc::new(ForgeStorage::connect_from_env().await?);
        storage.ping().await?;

        let org = forge_storage::repos::organizations::ensure_default(storage.pool()).await?;
        forge_storage::repos::billing::ensure_free(storage.pool(), &org.id).await?;

        if std::env::var("FORGE_ENV")
            .map(|v| v == "development")
            .unwrap_or(true)
        {
            let token =
                std::env::var("FORGE_DEV_API_TOKEN").unwrap_or_else(|_| "forge-dev-local".into());
            let hash = crate::auth::hash_token(&token);
            forge_storage::repos::tokens::insert_dev_token(
                storage.pool(),
                &org.id,
                &hash,
                "development",
                forge_storage::Role::Admin,
            )
            .await?;
            tracing::warn!("development API token active (set FORGE_DEV_API_TOKEN to override)");
        }

        Ok(Self {
            storage,
            cloud: forge_storage::cloud_mode(),
            engines: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Each organization gets its own engine, run store, and secret manager.
    /// This keeps in-process M2 state isolated even before durable engine storage exists.
    pub fn engine_for_org(&self, organization_id: &str) -> Arc<RunEngine> {
        let mut engines = self.engines.lock().expect("organization engine lock");
        engines
            .entry(organization_id.to_owned())
            .or_insert_with(|| {
                let store = Arc::new(MemoryStore::new());
                let secrets = Arc::new(SecretsManager::new());
                Arc::new(RunEngine::new(store, secrets, RunEngineConfig::default()))
            })
            .clone()
    }
}
