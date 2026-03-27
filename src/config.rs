use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shutdown_timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct LinkConfig {
    pub max_retries: usize,
    pub default_cache_control: String,
    pub query_timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub link: LinkConfig,
    pub log_level: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL must be set in environment".to_string())?;

        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        let max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .and_then(|c| c.parse().ok())
            .unwrap_or(12);

        let min_connections = std::env::var("DB_MIN_CONNECTIONS")
            .ok()
            .and_then(|c| c.parse().ok())
            .unwrap_or(2);

        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "lnkshort=debug".to_string());

        let max_retries = std::env::var("LINK_MAX_RETRIES")
            .ok()
            .and_then(|r| r.parse().ok())
            .unwrap_or(3);

        let query_timeout_ms = std::env::var("QUERY_TIMEOUT_MS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(300);

        let default_cache_control = std::env::var("DEFAULT_CACHE_CONTROL").unwrap_or_else(|_| {
            "public, max-age=300, s-maxage=300, state-while-revalidate=300, stale-if-error=300"
                .to_string()
        });

        Ok(Self {
            server: ServerConfig {
                host,
                port,
                shutdown_timeout: Duration::from_secs(5),
            },
            database: DatabaseConfig {
                url: database_url,
                max_connections,
                min_connections,
                acquire_timeout: Duration::from_secs(5),
            },
            link: LinkConfig {
                max_retries,
                default_cache_control,
                query_timeout: Duration::from_millis(query_timeout_ms),
            },
            log_level,
        })
    }
}
