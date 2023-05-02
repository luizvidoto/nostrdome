use nostr_sdk::secp256k1::XOnlyPublicKey;

use crate::db::DbRelay;

#[derive(Debug, Clone)]
pub enum NostrClientEvent {
    NostrConnected,
    NostrDisconnected,
    RelaysConnected,
    GotRelayServer(Option<nostr_sdk::Relay>),
    GotOwnEvents(Vec<nostr_sdk::Event>),
    GotPublicKey(XOnlyPublicKey),
    RelayMessage(nostr_sdk::RelayMessage),
    Shutdown,
    RelayConnected(DbRelay),
    UpdatedRelay,
    ChannelCreated(nostr_sdk::EventId),
}
