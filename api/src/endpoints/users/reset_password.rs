use axum::Json;
use axum::extract::State;
use validator::Validate;

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::models::{MessageResponse, ResetPasswordRequest};

/// Set a new password from a reset token
///
/// Consumes the reset token and revokes every existing session of the account.
#[utoipa::path(
    post,
    path = "/password/reset",
    operation_id = "resetPassword",
    tag = super::TAG,
    request_body = ResetPasswordRequest,
    responses(
        (status = OK, description = "Password updated", body = MessageResponse),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
        (status = UNAUTHORIZED, description = "Reset token is unknown, expired or already used", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> ApiResult<Json<MessageResponse>> {
    payload.validate()?;

    state
        .login
        .reset_password(&payload.token, &payload.password)
        .await?;

    Ok(Json(MessageResponse::new(
        "Password updated. Please sign in again.",
    )))
}
