use std::collections::HashMap;

use futures::channel::mpsc;
use futures_util::SinkExt;
use nostr::{RelayMessage, SubscriptionId};
use ns_client::NotificationEvent;

use crate::{db::Database, net::nostr_events::SubscriptionType};

use super::{BackEndInput, BackendEvent};

pub struct BackendState {
    pub db_client: Option<Database>,
    pub req_client: Option<reqwest::Client>,
    pub sender: mpsc::UnboundedSender<BackEndInput>,
    pub receiver: mpsc::UnboundedReceiver<BackEndInput>,
    pub subscriptions: HashMap<SubscriptionId, SubscriptionType>,
}
impl BackendState {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded();

        let mut subscriptions = HashMap::new();
        for sub_type in SubscriptionType::ALL.iter() {
            subscriptions.insert(SubscriptionId::new(sub_type.to_string()), *sub_type);
        }

        Self {
            db_client: None,
            req_client: None,
            sender,
            receiver,
            subscriptions,
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
            NotificationEvent::RelayMessage(relay_url, relay_msg) => {
                Some(BackEndInput::StoreRelayMessage((relay_url, relay_msg)))
            }
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
