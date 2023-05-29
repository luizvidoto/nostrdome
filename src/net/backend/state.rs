use futures::channel::mpsc;
use futures_util::SinkExt;
use nostr::RelayMessage;
use ns_client::NotificationEvent;

use crate::db::Database;

use super::{BackEndInput, BackendEvent};

pub struct BackendState {
    pub db_client: Option<Database>,
    pub req_client: Option<reqwest::Client>,
    pub sender: mpsc::UnboundedSender<BackEndInput>,
    pub receiver: mpsc::UnboundedReceiver<BackEndInput>,
}
impl BackendState {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            db_client: None,
            req_client: None,
            sender,
            receiver,
        }
    }

    pub fn with_db(mut self, db_client: Database) -> Self {
        self.db_client = Some(db_client);
        self
    }
    pub fn with_req(mut self, req_client: reqwest::Client) -> Self {
        self.req_client = Some(req_client);
        self
    }

    pub async fn logout(self) {
        if let Some(db) = self.db_client {
            tracing::info!("Database Logging out");
            db.pool.close().await;
            db.cache_pool.close().await;
        }
    }

    pub async fn process_notification(
        &mut self,
        notification: NotificationEvent,
    ) -> Option<BackendEvent> {
        let backend_input_opt = match notification {
            NotificationEvent::RelayTerminated(url) => {
                tracing::debug!("Relay terminated - {}", url);
                None
            }
            NotificationEvent::RelayMessage(relay_url, relay_msg) => match relay_msg {
                RelayMessage::Event {
                    subscription_id,
                    event,
                } => Some(BackEndInput::StoreConfirmedEvent((relay_url, *event))),
                other => Some(BackEndInput::StoreRelayMessage((relay_url, other))),
            },
            NotificationEvent::SentSubscription(url, sub_id) => {
                tracing::debug!("Sent subscription to {} - id: {}", url, sub_id);
                None
            }
            NotificationEvent::SentEvent(url, event_hash) => {
                tracing::debug!("Sent event to {} - hash: {}", url, event_hash);
                None
            }
        };

        if let Some(input) = backend_input_opt {
            if let Err(e) = self.sender.send(input).await {
                tracing::error!("{}", e);
            }
        }

        None
    }
}
