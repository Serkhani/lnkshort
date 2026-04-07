use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, NotSet, QueryFilter,
    QuerySelect, Set,
};

use crate::models::stats::{self, Entity as StatsEntity};
use crate::models::{CreateStat, LinkStats};

#[derive(Clone, Default)]
pub struct StatsRepository;

impl StatsRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn record_stat(
        &self,
        db: &DatabaseConnection,
        stat: &CreateStat,
    ) -> Result<(), DbErr> {
        let active_model = stats::ActiveModel {
            id: NotSet,
            link_id: Set(stat.link_id.clone()),
            referer: Set(stat.referer.clone()),
            user_agent: Set(stat.user_agent.clone()),
        };
        active_model.insert(db).await?;
        Ok(())
    }

    pub async fn get_stats_by_link_id(
        &self,
        db: &DatabaseConnection,
        link_id: &str,
    ) -> Result<Vec<LinkStats>, DbErr> {
        StatsEntity::find()
            .filter(stats::Column::LinkId.eq(link_id))
            .select_only()
            .column_as(stats::Column::Id.count(), "amount")
            .column(stats::Column::Referer)
            .column(stats::Column::UserAgent)
            .group_by(stats::Column::LinkId)
            .group_by(stats::Column::Referer)
            .group_by(stats::Column::UserAgent)
            .into_model::<LinkStats>()
            .all(db)
            .await
    }
}
