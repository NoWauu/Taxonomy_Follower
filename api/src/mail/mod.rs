mod logging;
mod smtp;

use std::sync::Arc;

use async_trait::async_trait;

use crate::config::MailConfig;

pub use logging::LoggingMailProvider;
pub use smtp::SmtpMailProvider;

pub struct Email {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: String,
}

#[async_trait]
pub trait MailProvider: Send + Sync + 'static {
    async fn send(&self, email: Email) -> anyhow::Result<()>;
}

pub fn from_config(config: &MailConfig) -> anyhow::Result<Arc<dyn MailProvider>> {
    match &config.smtp {
        Some(smtp) => Ok(Arc::new(SmtpMailProvider::new(smtp, &config.sender)?)),
        None => Ok(Arc::new(LoggingMailProvider::new())),
    }
}
