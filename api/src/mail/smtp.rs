use anyhow::Context;
use async_trait::async_trait;
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::config::{SmtpConfig, SmtpEncryption};

use super::{Email, MailProvider};

pub struct SmtpMailProvider {
    sender: Mailbox,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpMailProvider {
    pub fn new(config: &SmtpConfig, sender: &str) -> anyhow::Result<Self> {
        let sender: Mailbox = sender
            .parse()
            .with_context(|| format!("invalid SMTP_FROM address `{sender}`"))?;

        let mut transport = match config.encryption {
            SmtpEncryption::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .context("failed to build the SMTP relay")?,
            SmtpEncryption::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                    .context("failed to build the STARTTLS SMTP relay")?
            }
            SmtpEncryption::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            }
        }
        .port(config.port);

        if let Some(username) = &config.username {
            let password = config.password.clone().unwrap_or_default();
            transport = transport.credentials(Credentials::new(username.clone(), password));
        }

        Ok(Self {
            sender,
            transport: transport.build(),
        })
    }
}

#[async_trait]
impl MailProvider for SmtpMailProvider {
    async fn send(&self, email: Email) -> anyhow::Result<()> {
        let recipient: Mailbox = email
            .to
            .parse()
            .with_context(|| format!("invalid recipient address `{}`", email.to))?;

        let message = Message::builder()
            .from(self.sender.clone())
            .to(recipient)
            .subject(&email.subject)
            .multipart(MultiPart::alternative_plain_html(
                email.text_body,
                email.html_body,
            ))
            .context("failed to build the email")?;

        self.transport
            .send(message)
            .await
            .context("failed to send email")?;

        tracing::info!(to = %email.to, subject = %email.subject, "email sent");

        Ok(())
    }
}
