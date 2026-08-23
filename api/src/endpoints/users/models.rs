use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A user account as exposed to API consumers.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UserRecord> for User {
    fn from(record: UserRecord) -> Self {
        Self {
            id: record.id,
            email: record.email,
            display_name: record.display_name,
            email_verified: record.email_verified,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email(message = "must be a valid email address"))]
    #[validate(length(max = 320, message = "must be at most 320 characters"))]
    #[schema(example = "ada@example.com", max_length = 320, format = Email)]
    pub email: String,

    #[validate(length(min = 12, max = 128, message = "must be between 12 and 128 characters"))]
    #[schema(min_length = 12, max_length = 128, format = Password)]
    pub password: String,

    #[validate(length(min = 1, max = 128, message = "must be between 1 and 128 characters"))]
    #[schema(example = "Ada Lovelace", min_length = 1, max_length = 128)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "must be a valid email address"))]
    #[schema(example = "ada@example.com", format = Email)]
    pub email: String,

    #[validate(length(min = 1, message = "must not be empty"))]
    #[schema(min_length = 1, format = Password)]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefreshRequest {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyTokenRequest {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub token: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ForgotPasswordRequest {
    #[validate(email(message = "must be a valid email address"))]
    #[schema(example = "ada@example.com", format = Email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub token: String,

    #[validate(length(min = 12, max = 128, message = "must be between 12 and 128 characters"))]
    #[schema(min_length = 12, max_length = 128, format = Password)]
    pub password: String,
}

/// A freshly issued access token together with its refresh token.
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    #[schema(value_type = String, example = "Bearer")]
    pub token_type: &'static str,
    pub expires_in: i64,
    pub expires_at: DateTime<Utc>,
    pub refresh_token_expires_at: DateTime<Utc>,
}

/// The authenticated user plus the tokens issued for them.
///
/// `tokens` is flattened, so the JSON body carries the token fields at the top level.
#[derive(Debug, Serialize, ToSchema)]
pub struct Session {
    pub user: User,
    #[serde(flatten)]
    pub tokens: TokenPair,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyTokenResponse {
    pub valid: bool,
    pub user_id: Uuid,
    pub email: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    #[schema(value_type = String, example = "logged out")]
    pub message: &'static str,
}

impl MessageResponse {
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }
}
