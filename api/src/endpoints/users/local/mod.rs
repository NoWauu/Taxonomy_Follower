mod emails;
mod password;
mod repository;
mod token;

use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{ApiError, ApiResult};
use crate::mail::MailProvider;

use super::models::{Session, TokenPair, UserRecord, normalize_email};
use super::provider::{Credentials, LoginProvider, NewAccount, VerifiedIdentity};

pub use token::strip_bearer_prefix;

const MAX_PASSWORD_RESETS_PER_HOUR: i64 = 5;

pub struct LocalLoginProvider {
    db: sqlx::PgPool,
    config: Config,
    mailer: Arc<dyn MailProvider>,
}

impl LocalLoginProvider {
    pub fn new(db: sqlx::PgPool, config: Config, mailer: Arc<dyn MailProvider>) -> Self {
        Self { db, config, mailer }
    }

    async fn issue_tokens(&self, user: &UserRecord) -> ApiResult<TokenPair> {
        let (access_token, expires_at) = token::sign_access_token(&self.config, user)?;

        let refresh = token::generate_opaque_token();
        let refresh_lifetime = chrono::Duration::from_std(self.config.refresh_token_ttl)
            .context("refresh token TTL does not fit in a chrono duration")?;
        let refresh_token_expires_at = Utc::now() + refresh_lifetime;

        repository::insert_refresh_token(
            &self.db,
            user.id,
            &refresh.hash,
            refresh_token_expires_at,
        )
        .await?;

        Ok(TokenPair {
            access_token,
            refresh_token: refresh.raw,
            token_type: token::TOKEN_TYPE,
            expires_in: self.config.access_token_ttl.as_secs() as i64,
            expires_at,
            refresh_token_expires_at,
        })
    }

    async fn send_password_reset_email(&self, user: &UserRecord) -> ApiResult<()> {
        let since = Utc::now() - chrono::Duration::hours(1);
        let recent_requests =
            repository::count_recent_password_reset_requests(&self.db, user.id, since).await?;

        if recent_requests >= MAX_PASSWORD_RESETS_PER_HOUR {
            tracing::warn!(user_id = %user.id, "password reset throttled");
            return Ok(());
        }

        repository::invalidate_password_reset_tokens(&self.db, user.id).await?;

        let reset = token::generate_opaque_token();
        let lifetime = chrono::Duration::from_std(self.config.password_reset_ttl)
            .map_err(anyhow::Error::from)?;
        let expires_at = Utc::now() + lifetime;

        repository::insert_password_reset_token(&self.db, user.id, &reset.hash, expires_at).await?;

        let reset_link = format!("{}/reset-password?token={}", self.config.app_url, reset.raw);
        let email = emails::password_reset(&user.email, &reset_link, lifetime.num_minutes());
        let mailer = Arc::clone(&self.mailer);
        let user_id = user.id;

        tokio::spawn(async move {
            match mailer.send(email).await {
                Ok(()) => tracing::info!(%user_id, "password reset email dispatched"),
                Err(error) => {
                    tracing::error!(error = ?error, %user_id, "failed to send password reset email")
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl LoginProvider for LocalLoginProvider {
    async fn register(&self, account: NewAccount) -> ApiResult<Session> {
        let email = normalize_email(&account.email);
        password::reject_if_predictable(&account.password, &email)?;

        let password_hash = password::hash(account.password).await?;
        let display_name = account
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());

        let user = repository::insert_user(&self.db, &email, &password_hash, display_name).await?;
        let tokens = self.issue_tokens(&user).await?;

        tracing::info!(user_id = %user.id, "user registered");

        Ok(Session {
            user: user.into(),
            tokens,
        })
    }

    async fn login(&self, credentials: Credentials) -> ApiResult<Session> {
        let email = normalize_email(&credentials.email);
        let rejection = || ApiError::Unauthorized("invalid email or password");

        let Some(user) = repository::find_user_by_email(&self.db, &email).await? else {
            password::spend_time_as_if_verifying(credentials.password).await;
            return Err(rejection());
        };

        if !password::matches(credentials.password, user.password_hash.clone()).await? {
            tracing::info!(user_id = %user.id, "failed login attempt");
            return Err(rejection());
        }

        let tokens = self.issue_tokens(&user).await?;

        tracing::info!(user_id = %user.id, "user logged in");

        Ok(Session {
            user: user.into(),
            tokens,
        })
    }

    async fn refresh(&self, refresh_token: &str) -> ApiResult<TokenPair> {
        let token_hash = token::hash_opaque_token(refresh_token.trim());

        let user_id = repository::consume_refresh_token(&self.db, &token_hash)
            .await?
            .ok_or(ApiError::Unauthorized("invalid or expired refresh token"))?;

        let user = repository::find_user_by_id(&self.db, user_id)
            .await?
            .ok_or(ApiError::Unauthorized("the account no longer exists"))?;

        let tokens = self.issue_tokens(&user).await?;

        tracing::debug!(%user_id, "refresh token rotated");

        Ok(tokens)
    }

    async fn verify_access_token(&self, access_token: &str) -> ApiResult<VerifiedIdentity> {
        let claims = token::decode_access_token(&self.config, access_token)?;

        let user = repository::find_user_by_id(&self.db, claims.sub)
            .await?
            .ok_or(ApiError::Unauthorized("the account no longer exists"))?;

        Ok(VerifiedIdentity {
            user: user.into(),
            issued_at: claims.issued_at(),
            expires_at: claims.expires_at(),
        })
    }

    async fn logout(&self, refresh_token: &str) -> ApiResult<()> {
        let token_hash = token::hash_opaque_token(refresh_token.trim());
        repository::revoke_refresh_token(&self.db, &token_hash).await?;

        Ok(())
    }

    async fn logout_everywhere(&self, user_id: Uuid) -> ApiResult<u64> {
        let revoked = repository::revoke_all_refresh_tokens(&self.db, user_id).await?;
        tracing::info!(%user_id, revoked, "all sessions revoked");

        Ok(revoked)
    }

    async fn request_password_reset(&self, email: &str) -> ApiResult<()> {
        let email = normalize_email(email);

        if let Some(user) = repository::find_user_by_email(&self.db, &email).await? {
            self.send_password_reset_email(&user).await?;
        }

        Ok(())
    }

    async fn reset_password(&self, reset_token: &str, new_password: &str) -> ApiResult<()> {
        let token_hash = token::hash_opaque_token(reset_token.trim());
        let expired_or_unknown =
            || ApiError::BadRequest("this reset link is invalid, already used, or expired".into());

        let user_id = repository::find_user_for_active_password_reset_token(&self.db, &token_hash)
            .await?
            .ok_or_else(expired_or_unknown)?;

        let user = repository::find_user_by_id(&self.db, user_id)
            .await?
            .ok_or(ApiError::NotFound("the account no longer exists"))?;

        password::reject_if_predictable(new_password, &user.email)?;
        let password_hash = password::hash(new_password.to_string()).await?;

        repository::consume_password_reset_token(&self.db, &token_hash)
            .await?
            .ok_or_else(expired_or_unknown)?;

        repository::update_password(&self.db, user.id, &password_hash).await?;

        let revoked = repository::revoke_all_refresh_tokens(&self.db, user.id).await?;
        tracing::info!(user_id = %user.id, revoked, "password reset completed");

        Ok(())
    }
}
