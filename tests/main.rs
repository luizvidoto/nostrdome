use common::make_channel_creation_event;
use nostrtalk::{
    db::{upgrade_cache_db, upgrade_db, ChannelCache, Database, DbContact},
    types::{BackendState, ChannelMetadata},
};
use once_cell::sync::Lazy;
use sqlx::SqlitePool;
use tempfile::NamedTempFile;

mod common;
mod kind;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| nostrtalk::setup_logger());

pub struct TestApp {
    pub backend: BackendState,
    pub keys: nostr::Keys,
}
impl TestApp {
    pub fn pool(&self) -> &SqlitePool {
        &self.backend.pool()
    }
    pub fn cache_pool(&self) -> &SqlitePool {
        &self.backend.cache_pool()
    }
    pub async fn insert_contacts(&self, contacts: Vec<nostr::Contact>) {
        for contact in contacts {
            // let db_c: DbContact = contact.into();
            DbContact::insert(self.pool(), &contact.pk).await.unwrap();
        }
    }
    pub async fn insert_channel_cache(&self, creation_event: nostr::Event) -> ChannelCache {
        let cache = ChannelCache::fetch_insert(self.cache_pool(), &creation_event)
            .await
            .unwrap();
        cache
    }
    pub async fn insert_random_channel_cache(&self) -> ChannelCache {
        let metadata = ChannelMetadata::new().name("test_channel");
        let creation_event = make_channel_creation_event(&self.keys, &metadata);
        self.insert_channel_cache(creation_event).await
    }
}

pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let keys = nostr::Keys::generate();
    let db_client = configure_database().await;
    let req_client = reqwest::Client::new();
    let backend = BackendState::new(
        db_client,
        req_client,
        ns_client::RelayPool::new(),
        Vec::new(),
        None,
    );
    let test_app = TestApp { backend, keys };

    test_app
}

async fn configure_database() -> Database {
    let pool = configure_pool().await;
    let cache_pool = configure_pool().await;

    upgrade_db(&pool).await.unwrap();
    upgrade_cache_db(&cache_pool).await.unwrap();

    Database { cache_pool, pool }
}

async fn configure_pool() -> SqlitePool {
    let tempfile = NamedTempFile::new().unwrap();
    let pool = SqlitePool::connect(tempfile.path().to_str().unwrap())
        .await
        .unwrap();
    pool
}
