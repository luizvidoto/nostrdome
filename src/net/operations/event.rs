use crate::{
    db::{DbEvent, DbRelayResponse},
    Error,
};
use nostr::Url;
use ns_client::RelayPool;
use sqlx::SqlitePool;

pub async fn confirmed_event(
    pool: &SqlitePool,
    url: &Url,
    ns_event: nostr::Event,
) -> Result<Option<DbEvent>, Error> {
    let mut db_event = DbEvent::confirmed_event(ns_event, url)?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    DbRelayResponse::insert_ok(pool, url, &db_event).await?;

    if rows_changed == 0 {
        //maybe this never happens because the database checks
        // the event_hash uniqueness and sends an error
        tracing::debug!("Event already in database");
        return Ok(None);
    }

    Ok(Some(db_event))
}

pub async fn insert_pending(
    ns_event: nostr::Event,
    pool: &SqlitePool,
    nostr: &RelayPool,
) -> Result<DbEvent, Error> {
    let mut pending_event = DbEvent::pending_event(ns_event.clone())?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &pending_event).await?;
    pending_event = pending_event.with_id(row_id);

    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Err(Error::DuplicatedEvent);
    }

    nostr.send_event(ns_event)?;

    Ok(pending_event)
}
