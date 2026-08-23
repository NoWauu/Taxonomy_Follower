use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::ApiResult;

use super::models::{Session, TokenPair, User};

pub struct NewAccount {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

pub struct Credentials {
    pub email: String,
    pub password: String,
}

pub struct VerifiedIdentity {
    pub user: User,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[async_trait]
pub trait LoginProvider: Send + Sync + 'static {
    async fn register(&self, account: NewAccount) -> ApiResult<Session>;

    async fn login(&self, credentials: Credentials) -> ApiResult<Session>;

    async fn refresh(&self, refresh_token: &str) -> ApiResult<TokenPair>;

    async fn verify_access_token(&self, access_token: &str) -> ApiResult<VerifiedIdentity>;

    async fn logout(&self, refresh_token: &str) -> ApiResult<()>;

    async fn logout_everywhere(&self, user_id: Uuid) -> ApiResult<u64>;

    async fn request_password_reset(&self, email: &str) -> ApiResult<()>;

    async fn reset_password(&self, reset_token: &str, new_password: &str) -> ApiResult<()>;
}
