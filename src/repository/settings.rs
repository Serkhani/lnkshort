use sqlx::PgPool;

use crate::models::Setting;

#[derive(Clone, Default)]
pub struct SettingsRepository;

impl SettingsRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_default_settings(&self, pool: &PgPool) -> Result<Setting, sqlx::Error> {
        sqlx::query_as!(
            Setting,
            "select id, encrypted_global_api_key from settings where id = $1",
            "DEFAULT_SETTINGS"
        )
        .fetch_one(pool)
        .await
    }
}
