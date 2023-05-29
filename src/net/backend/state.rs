use std::collections::HashMap;

use futures::channel::mpsc;
use futures_util::SinkExt;
use nostr::SubscriptionId;
use nostr_sdk::RelayPoolNotification;

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
        notification: RelayPoolNotification,
    ) -> Option<BackendEvent> {
        let backend_input_opt = match notification {
            RelayPoolNotification::Message(relay_url, relay_msg) => {
                Some(BackEndInput::StoreRelayMessage((relay_url, relay_msg)))
            }
            RelayPoolNotification::Event(_, _) => None,
            RelayPoolNotification::Shutdown => None,
        };

        if let Some(input) = backend_input_opt {
            if let Err(e) = self.sender.send(input).await {
                tracing::error!("{}", e);
            }
        }

        None
    }
}
