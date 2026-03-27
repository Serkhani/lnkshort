use sqlx::PgPool;

use crate::models::{CreateStat, LinkStats};

#[derive(Clone, Default)]
pub struct StatsRepository;

impl StatsRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn record_stat(&self, pool: &PgPool, stat: &CreateStat) -> Result<(), sqlx::Error> {
        sqlx::query("insert into lnkstats (link_id, user_agent, referer) values ($1, $2, $3)")
            .bind(&stat.link_id)
            .bind(&stat.user_agent)
            .bind(&stat.referer)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_stats_by_link_id(
        &self,
        pool: &PgPool,
        link_id: &str,
    ) -> Result<Vec<LinkStats>, sqlx::Error> {
        sqlx::query_as!(
            LinkStats,
            "select count(*) as amount, referer, user_agent from lnkstats where link_id=$1 group by link_id, referer, user_agent",
            link_id
        )
        .fetch_all(pool)
        .await
    }
}
