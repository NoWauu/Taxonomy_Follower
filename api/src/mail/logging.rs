use async_trait::async_trait;

use super::{Email, MailProvider};

pub struct LoggingMailProvider;

impl LoggingMailProvider {
    pub fn new() -> Self {
        tracing::warn!("SMTP_HOST is not set: outgoing emails will be logged, not sent");
        Self
    }
}

impl Default for LoggingMailProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MailProvider for LoggingMailProvider {
    async fn send(&self, email: Email) -> anyhow::Result<()> {
        tracing::warn!(
            to = %email.to,
            subject = %email.subject,
            body = %email.text_body,
            "SMTP disabled, email not sent",
        );

        Ok(())
    }
}
