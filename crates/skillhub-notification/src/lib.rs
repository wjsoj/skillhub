//! Notification dispatch (email / webhook / in-app).
//!
//! Initial implementation: log-only notifier. Real backends slot
//! in by implementing `Notifier`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub recipient: Uuid,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, notification: &Notification) -> anyhow::Result<()>;
}

pub struct LogNotifier;

#[async_trait]
impl Notifier for LogNotifier {
    async fn send(&self, notification: &Notification) -> anyhow::Result<()> {
        tracing::info!(?notification, "notification dispatched");
        Ok(())
    }
}
