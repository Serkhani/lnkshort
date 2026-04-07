use sea_orm::FromQueryResult;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "lnkstats")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub link_id: String,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::link::Entity",
        from = "Column::LinkId",
        to = "super::link::Column::Id"
    )]
    Link,
}

impl Related<super::link::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Link.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct LinkStats {
    pub amount: Option<i64>,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateStat {
    pub link_id: String,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
}
