use std::sync::Arc;
use sqlx::PgPool;

use crate::config::LinkConfig;
use crate::error::AppError;
use crate::models::{CreateStat, LinkStats};
use crate::repository::StatsRepository;

#[derive(Clone)]
pub struct StatsService {
    repo: StatsRepository,
    config: Arc<LinkConfig>,
}

impl StatsService {
    pub fn new(repo: StatsRepository, config: Arc<LinkConfig>) -> Self {
        Self { repo, config }
    }

    pub async fn record_stat(
        &self,
        pool: &PgPool,
        link_id: String,
        user_agent: Option<String>,
        referer: Option<String>,
    ) {
        let stat = CreateStat {
            link_id: link_id.clone(),
            user_agent: user_agent.clone(),
            referer: referer.clone(),
        };

        let result = tokio::time::timeout(
            self.config.query_timeout,
            self.repo.record_stat(pool, &stat),
        )
        .await;

        match result {
            Err(elapsed) => tracing::error!("Could not save stat in time: {}", elapsed),
            Ok(Err(err)) => tracing::error!("Error while saving stat: {}", err),
            Ok(Ok(())) => tracing::debug!(
                "id: {}, referer: {:?}, user_agent: {:?} saved",
                link_id,
                referer.as_deref().unwrap_or_default(),
                user_agent.as_deref().unwrap_or_default()
            ),
        }
    }

    pub async fn get_stats(&self, pool: &PgPool, link_id: &str) -> Result<Vec<LinkStats>, AppError> {
        let stats = tokio::time::timeout(
            self.config.query_timeout,
            self.repo.get_stats_by_link_id(pool, link_id),
        )
        .await
        .map_err(|e| AppError::Timeout(e.to_string()))?
        .map_err(AppError::Database)?;

        tracing::debug!("Link stats for id: {} fetched", link_id);
        Ok(stats)
    }
}
