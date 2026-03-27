use std::sync::Arc;
use metrics::counter;
use sha3::{Digest, Sha3_256};
use sqlx::PgPool;

use crate::config::LinkConfig;
use crate::error::AppError;
use crate::repository::SettingsRepository;

#[derive(Clone)]
pub struct AuthService {
    repo: SettingsRepository,
    config: Arc<LinkConfig>,
}

impl AuthService {
    pub fn new(repo: SettingsRepository, config: Arc<LinkConfig>) -> Self {
        Self { repo, config }
    }

    /// Constant-time slice comparison to prevent timing attacks
    fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter().zip(b.iter()).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
    }

    pub async fn verify_api_key(
        &self,
        pool: &PgPool,
        api_key: &str,
        request_uri: &str,
    ) -> Result<(), AppError> {
        let labels = [("uri", format!("{}!", request_uri))];

        let setting = tokio::time::timeout(
            self.config.query_timeout,
            self.repo.get_default_settings(pool),
        )
        .await
        .map_err(|e| AppError::Timeout(e.to_string()))?
        .map_err(AppError::Database)?;

        let mut hasher = Sha3_256::new();
        hasher.update(api_key.as_bytes());
        let provided_hash = hex::encode(hasher.finalize());

        let is_valid = Self::constant_time_compare(
            setting.encrypted_global_api_key.as_bytes(),
            provided_hash.as_bytes(),
        );

        if !is_valid {
            tracing::error!("unauthenticated call to API: Incorrect key supplied");
            counter!("unauthenticated_calls_count", &labels).increment(1);
            return Err(AppError::Unauthorized("Unauthorised".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare_matches() {
        let a = b"secret-token-hash-123456";
        let b = b"secret-token-hash-123456";
        assert!(AuthService::constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_compare_mismatches() {
        let a = b"secret-token-hash-123456";
        let b = b"secret-token-hash-654321";
        assert!(!AuthService::constant_time_compare(a, b));

        let c = b"different-length";
        assert!(!AuthService::constant_time_compare(a, c));
    }
}
