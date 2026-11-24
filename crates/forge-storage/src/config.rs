use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    Sqlite,
    Postgres,
}

impl DatabaseBackend {
    pub fn detect(url: &str) -> Self {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Self::Postgres
        } else {
            Self::Sqlite
        }
    }
}

pub fn database_url_from_env() -> String {
    env::var("FORGE_DATABASE_URL").unwrap_or_else(|_| "sqlite://forge.db?mode=rwc".to_string())
}

pub fn cloud_mode() -> bool {
    env::var("FORGE_CLOUD")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
