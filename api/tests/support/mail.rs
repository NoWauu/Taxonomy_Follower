//! Mail provider used by the suite: nothing leaves the process, everything is
//! kept so a scenario can assert on it.
//!
//! The real provider sends password reset emails from a spawned task, so the
//! mail a scenario waits for lands here slightly after the request returns;
//! [`CapturingMailProvider::wait_for`] is what bridges that gap.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use api::mail::{Email, MailProvider};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub text_body: String,
}

#[derive(Default)]
pub struct CapturingMailProvider {
    sent: Mutex<Vec<SentEmail>>,
}

impl CapturingMailProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn to(&self, recipient: &str) -> Vec<SentEmail> {
        self.sent
            .lock()
            .expect("mailbox lock")
            .iter()
            .filter(|email| email.to.eq_ignore_ascii_case(recipient))
            .cloned()
            .collect()
    }

    /// Waits up to `timeout` for a mail addressed to `recipient`.
    pub async fn wait_for(&self, recipient: &str, timeout: Duration) -> Option<SentEmail> {
        let deadline = Instant::now() + timeout;

        loop {
            if let Some(email) = self.to(recipient).into_iter().next_back() {
                return Some(email);
            }

            if Instant::now() >= deadline {
                return None;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

#[async_trait]
impl MailProvider for CapturingMailProvider {
    async fn send(&self, email: Email) -> anyhow::Result<()> {
        self.sent.lock().expect("mailbox lock").push(SentEmail {
            to: email.to,
            subject: email.subject,
            text_body: email.text_body,
        });

        Ok(())
    }
}
