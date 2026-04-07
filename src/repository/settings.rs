use sea_orm::{DatabaseConnection, DbErr, EntityTrait};

use crate::models::Setting;
use crate::models::settings::Entity as SettingsEntity;

#[derive(Clone, Default)]
pub struct SettingsRepository;

impl SettingsRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_default_settings(&self, db: &DatabaseConnection) -> Result<Setting, DbErr> {
        SettingsEntity::find_by_id("DEFAULT_SETTINGS")
            .one(db)
            .await?
            .ok_or_else(|| DbErr::RecordNotFound("DEFAULT_SETTINGS not found".to_string()))
    }
}
