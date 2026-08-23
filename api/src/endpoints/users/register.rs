use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use validator::Validate;

use crate::AppState;
use crate::error::{ApiResult, ErrorResponse};

use super::models::{RegisterRequest, Session};
use super::provider::NewAccount;

/// Create a new account
///
/// Registers a local account and immediately signs it in, returning the user
/// together with a fresh token pair.
#[utoipa::path(
    post,
    path = "/register",
    operation_id = "registerUser",
    tag = super::TAG,
    request_body = RegisterRequest,
    responses(
        (status = CREATED, description = "Account created and signed in", body = Session),
        (status = BAD_REQUEST, description = "Payload failed validation", body = ErrorResponse),
        (status = CONFLICT, description = "Email is already registered", body = ErrorResponse),
    ),
)]
pub async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<Session>)> {
    payload.validate()?;

    let session = state
        .login
        .register(NewAccount {
            email: payload.email,
            password: payload.password,
            display_name: payload.display_name,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(session)))
}
