use base64::Engine;
use base64::engine::general_purpose;
use metrics::counter;
use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;
use url::Url;

use crate::config::LinkConfig;
use crate::error::AppError;
use crate::models::Link;
use crate::repository::LinkRepository;

#[derive(Clone)]
pub struct LinkService {
    repo: LinkRepository,
    config: Arc<LinkConfig>,
}

impl LinkService {
    pub fn new(repo: LinkRepository, config: Arc<LinkConfig>) -> Self {
        Self { repo, config }
    }

    pub fn generate_id() -> String {
        let randnum = rand::random_range(0..u64::MAX);
        general_purpose::URL_SAFE_NO_PAD.encode(randnum.to_string())
    }

    pub fn validate_url(raw_url: &str) -> Result<String, AppError> {
        let parsed =
            Url::parse(raw_url).map_err(|_| AppError::InvalidUrl("url malformed".to_string()))?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(AppError::InvalidUrl("url malformed".to_string()));
        }
        Ok(parsed.to_string())
    }

    pub async fn get_link(&self, db: &DatabaseConnection, id: &str) -> Result<Link, AppError> {
        let link_opt =
            tokio::time::timeout(self.config.query_timeout, self.repo.find_by_id(db, id))
                .await
                .map_err(|e| AppError::Timeout(e.to_string()))?
                .map_err(AppError::Database)?;

        link_opt.ok_or_else(|| AppError::NotFound("Not found".to_string()))
    }

    fn is_unique_violation(err: &DbErr) -> bool {
        match err {
            DbErr::Exec(sea_orm::RuntimeErr::SqlxError(sea_orm::sqlx::Error::Database(db_err)))
            | DbErr::Query(sea_orm::RuntimeErr::SqlxError(sea_orm::sqlx::Error::Database(
                db_err,
            ))) => {
                db_err.kind() == sea_orm::sqlx::error::ErrorKind::UniqueViolation
                    || db_err.code().as_deref() == Some("23505")
            }
            _ => {
                let msg = err.to_string().to_lowercase();
                msg.contains("unique") || msg.contains("duplicate") || msg.contains("23505")
            }
        }
    }

    pub async fn create_link(
        &self,
        db: &DatabaseConnection,
        raw_url: &str,
    ) -> Result<Link, AppError> {
        let url = Self::validate_url(raw_url)?;

        for _ in 1..=self.config.max_retries {
            let new_link_id = Self::generate_id();
            let result = tokio::time::timeout(
                self.config.query_timeout,
                self.repo.create(db, &new_link_id, &url),
            )
            .await;

            match result {
                Ok(Ok(link)) => {
                    tracing::debug!(
                        "Created link with id {} targeting: {}",
                        link.id,
                        link.target_url
                    );
                    return Ok(link);
                }
                Ok(Err(err)) if Self::is_unique_violation(&err) => {
                    tracing::warn!(
                        "Collision detected for link id {}, retrying...",
                        new_link_id
                    );
                    continue;
                }
                Ok(Err(err)) => return Err(AppError::Database(err)),
                Err(elapsed) => return Err(AppError::Timeout(elapsed.to_string())),
            }
        }

        tracing::error!("Could not persist new short link. Exhausted all retries");
        counter!("exhausting_unique_entries").increment(1);
        Err(AppError::ExhaustedRetries)
    }

    pub async fn update_link(
        &self,
        db: &DatabaseConnection,
        id: &str,
        raw_url: &str,
    ) -> Result<Link, AppError> {
        let url = Self::validate_url(raw_url)?;

        let updated_link =
            tokio::time::timeout(self.config.query_timeout, self.repo.update(db, id, &url))
                .await
                .map_err(|e| AppError::Timeout(e.to_string()))?
                .map_err(|err| match err {
                    DbErr::RecordNotFound(_) => AppError::NotFound("Not found".to_string()),
                    other => AppError::Database(other),
                })?;

        tracing::debug!("Updated link with id {} to {}", id, url);
        Ok(updated_link)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_format() {
        let id = LinkService::generate_id();
        assert!(!id.is_empty());
        // URL-safe characters only
        assert!(
            id.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn test_validate_url_valid() {
        let valid_urls = [
            "https://example.com",
            "http://example.com/sub/path?param=123#anchor",
            "https://sub.domain.co.uk:8080/test",
        ];
        for url in valid_urls {
            let res = LinkService::validate_url(url);
            assert!(res.is_ok(), "Failed for valid URL: {}", url);
        }
    }

    #[test]
    fn test_validate_url_invalid() {
        let invalid_urls = ["not-a-url", "ftp://example.com", ""];
        for url in invalid_urls {
            let res = LinkService::validate_url(url);
            assert!(res.is_err(), "Should have failed for invalid URL: {}", url);
        }
    }
}
