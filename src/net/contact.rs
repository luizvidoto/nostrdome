use crate::{db::DbContact, error::Error};

use nostr_sdk::Keys;
use sqlx::SqlitePool;

pub async fn insert_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<DbContact, Error> {
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactInsert);
    }
    DbContact::insert(pool, &db_contact).await?;
    Ok(db_contact.to_owned())
}

pub async fn insert_batch_of_contacts(
    keys: &Keys,
    pool: &SqlitePool,
    db_contacts: &[DbContact],
) -> Result<(), Error> {
    for db_contact in db_contacts {
        if let Err(e) = insert_contact(keys, pool, db_contact).await {
            tracing::error!("{}", e);
        }
    }
    Ok(())
}
