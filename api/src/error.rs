use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;
use validator::ValidationErrors;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),

    #[error("validation failed")]
    Validation(#[from] ValidationErrors),

    #[error("{0}")]
    Unauthorized(&'static str),

    #[error("{0}")]
    NotFound(&'static str),

    #[error("{0}")]
    Conflict(&'static str),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Error payload returned by every failing endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Machine readable error code.
    #[schema(example = "validation_error")]
    pub error: &'static str,

    /// Human readable description of what went wrong.
    #[schema(example = "validation failed")]
    pub message: String,

    /// Per-field validation failures, only present on `validation_error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::Validation(_) => "validation_error",
            Self::Unauthorized(_) => "unauthorized",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(anyhow::Error::new(error).context("database query failed"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();

        let mut body = ErrorResponse {
            error: self.code(),
            message: self.to_string(),
            details: None,
        };

        match &self {
            ApiError::Validation(errors) => {
                body.details = Some(serde_json::json!(errors));
            }
            ApiError::Internal(error) => {
                tracing::error!(error = ?error, "request failed");
                body.message = "internal server error".to_string();
            }
            _ => {}
        }

        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
