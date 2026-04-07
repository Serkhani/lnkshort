use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set};

use crate::models::Link;
use crate::models::link::{self, Entity as LinkEntity};

#[derive(Clone, Default)]
pub struct LinkRepository;

impl LinkRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn find_by_id(
        &self,
        db: &DatabaseConnection,
        id: &str,
    ) -> Result<Option<Link>, DbErr> {
        LinkEntity::find_by_id(id).one(db).await
    }

    pub async fn create(
        &self,
        db: &DatabaseConnection,
        id: &str,
        target_url: &str,
    ) -> Result<Link, DbErr> {
        let active_model = link::ActiveModel {
            id: Set(id.to_string()),
            target_url: Set(target_url.to_string()),
        };
        active_model.insert(db).await
    }

    pub async fn update(
        &self,
        db: &DatabaseConnection,
        id: &str,
        target_url: &str,
    ) -> Result<Link, DbErr> {
        let existing = LinkEntity::find_by_id(id).one(db).await?;
        match existing {
            Some(model) => {
                let mut active: link::ActiveModel = model.into();
                active.target_url = Set(target_url.to_string());
                active.update(db).await
            }
            None => Err(DbErr::RecordNotFound(format!("Link '{}' not found", id))),
        }
    }
}
