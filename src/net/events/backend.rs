use crate::db::{DbEvent, DbRelay};

#[derive(Debug, Clone)]
pub enum BackEndEvent {
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    AddRelayToDb(DbRelay),
    DeleteRelayFromDb(DbRelay),
    InsertPendingEvent(nostr_sdk::Event),
    ReceivedEvent((nostr_sdk::Url, nostr_sdk::Event)),
    ReceivedRelayMessage((nostr_sdk::Url, nostr_sdk::RelayMessage)),
    PrepareClient {
        relays: Vec<DbRelay>,
        last_event: Option<DbEvent>,
    },
}
