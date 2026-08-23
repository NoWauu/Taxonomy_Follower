use axum::Json;
use axum::extract::State;
use validator::Validate;

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::extractor::AuthenticatedUser;
use super::models::{MessageResponse, RefreshRequest};

/// Revoke a single refresh token
#[utoipa::path(
    post,
    path = "/logout",
    operation_id = "logout",
    tag = super::TAG,
    request_body = RefreshRequest,
    responses(
        (status = OK, description = "Refresh token revoked", body = MessageResponse),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> ApiResult<Json<MessageResponse>> {
    payload.validate()?;

    state.login.logout(&payload.refresh_token).await?;

    Ok(Json(MessageResponse::new("logged out")))
}

/// Revoke every refresh token of the authenticated user
#[utoipa::path(
    post,
    path = "/logout/all",
    operation_id = "logoutEverywhere",
    tag = super::TAG,
    security(("bearer_auth" = [])),
    responses(
        (status = OK, description = "All sessions revoked", body = MessageResponse),
        (status = UNAUTHORIZED, description = "Missing or invalid access token", body = ErrorResponse),
    ),
)]
pub async fn everywhere(
    State(state): State<AppState>,
    AuthenticatedUser(user): AuthenticatedUser,
) -> ApiResult<Json<MessageResponse>> {
    state.login.logout_everywhere(user.id).await?;

    Ok(Json(MessageResponse::new("all sessions revoked")))
}
