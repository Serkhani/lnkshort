use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use metrics::counter;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    InvalidUrl(String),
    Unauthorized(String),
    Database(sqlx::Error),
    Timeout(String),
    ExhaustedRetries,
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "{}", msg),
            Self::InvalidUrl(msg) => write!(f, "{}", msg),
            Self::Unauthorized(msg) => write!(f, "{}", msg),
            Self::Database(err) => write!(f, "{}", err),
            Self::Timeout(msg) => write!(f, "{}", msg),
            Self::ExhaustedRetries => write!(f, "Internal server error"),
            Self::Internal(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(err) => Some(err),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

impl From<tokio::time::error::Elapsed> for AppError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::Timeout(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::InvalidUrl(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::ExhaustedRetries => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            AppError::Database(err) => {
                tracing::error!("Database error: {}", err);
                let labels = [("error", format!("{}!", err))];
                counter!("request_error", &labels).increment(1);
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
            AppError::Timeout(msg) => {
                tracing::error!("Operation timed out: {}", msg);
                let labels = [("error", format!("{}!", msg))];
                counter!("request_error", &labels).increment(1);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                let labels = [("error", format!("{}!", msg))];
                counter!("request_error", &labels).increment(1);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
        };

        (status, body).into_response()
    }
}

pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    tracing::error!("{}", err);
    let labels = [("error", format!("{}!", err))];
    counter!("request_error", &labels).increment(1);

    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
