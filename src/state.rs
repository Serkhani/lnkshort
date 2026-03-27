use std::sync::Arc;
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::repository::{LinkRepository, SettingsRepository, StatsRepository};
use crate::services::{AuthService, LinkService, StatsService};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<AppConfig>,
    pub link_service: LinkService,
    pub stats_service: StatsService,
    pub auth_service: AuthService,
}

impl AppState {
    pub fn new(db: PgPool, config: Arc<AppConfig>) -> Self {
        let link_repo = LinkRepository::new();
        let stats_repo = StatsRepository::new();
        let settings_repo = SettingsRepository::new();

        let link_config = Arc::new(config.link.clone());

        let link_service = LinkService::new(link_repo, link_config.clone());
        let stats_service = StatsService::new(stats_repo, link_config.clone());
        let auth_service = AuthService::new(settings_repo, link_config);

        Self {
            db,
            config,
            link_service,
            stats_service,
            auth_service,
        }
    }
}
