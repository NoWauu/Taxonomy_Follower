use axum::Json;
use axum::extract::State;
use validator::Validate;

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::models::{ForgotPasswordRequest, MessageResponse};

/// Request a password reset link
///
/// Always answers `200` so the endpoint cannot be used to probe which email
/// addresses have an account.
#[utoipa::path(
    post,
    path = "/password/forgot",
    operation_id = "forgotPassword",
    tag = super::TAG,
    request_body = ForgotPasswordRequest,
    responses(
        (status = OK, description = "Reset link sent if the account exists", body = MessageResponse),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> ApiResult<Json<MessageResponse>> {
    payload.validate()?;

    state.login.request_password_reset(&payload.email).await?;

    Ok(Json(MessageResponse::new(
        "If an account exists for this address, a reset link has been sent.",
    )))
}
