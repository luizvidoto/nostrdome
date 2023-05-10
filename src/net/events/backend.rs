use std::path::PathBuf;

use chrono::NaiveDateTime;
use futures::channel::mpsc;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Keys, Metadata, RelayMessage, Url};
use sqlx::SqlitePool;

use crate::{
    db::{
        self, DbChat, DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse, MessageStatus,
        UserConfig,
    },
    error::Error,
    net::{
        client::{download_image, ImageKind},
        events::nostr::NostrInput,
        process_async_with_event,
    },
    types::ChatMessage,
};

use super::{frontend::SpecificEvent, Event};

#[derive(Debug, Clone)]
pub enum BackEndInput {
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    AddRelayToDb(DbRelay),
    DeleteRelayFromDb(DbRelay),
    StorePendingEvent(nostr_sdk::Event),
    StoreEvent((nostr_sdk::Url, nostr_sdk::Event)),
    StoreRelayMessage((nostr_sdk::Url, nostr_sdk::RelayMessage)),
    LatestVersion(String),
    ProfilePictureDownloaded((XOnlyPublicKey, PathBuf)),
    ProfileBannerDownloaded((XOnlyPublicKey, PathBuf)),
    Shutdown,
    FinishedPreparingNostr,
    Error(String),
    Ok(Event),
}

pub async fn backend_processing(
    pool: &SqlitePool,
    keys: &Keys,
    input: BackEndInput,
    sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Event {
    match input {
        BackEndInput::Ok(event) => event,
        BackEndInput::Error(e) => Event::Error(e),
        // --- REQWEST ---
        BackEndInput::LatestVersion(version) => Event::LatestVersion(version),
        BackEndInput::Shutdown => Event::None,
        // --- TO DATABASE ---
        BackEndInput::FinishedPreparingNostr => Event::FinishedPreparing,
        BackEndInput::ProfilePictureDownloaded((public_key, path)) => {
            handle_profile_picture_update(keys, public_key, pool, path).await
        }
        BackEndInput::ProfileBannerDownloaded((public_key, path)) => {
            handle_profile_banner_update(keys, public_key, pool, path).await
        }
        BackEndInput::DeleteRelayFromDb(db_relay) => {
            process_async_with_event(db_delete_relay(&pool, db_relay)).await
        }
        BackEndInput::ToggleRelayRead((mut db_relay, read)) => {
            db_relay.read = read;
            match DbRelay::update(&pool, &db_relay).await {
                Ok(_) => Event::RelayUpdated(db_relay.clone()),
                Err(e) => Event::Error(e.to_string()),
            }
        }
        BackEndInput::ToggleRelayWrite((mut db_relay, write)) => {
            db_relay.write = write;
            match DbRelay::update(&pool, &db_relay).await {
                Ok(_) => Event::RelayUpdated(db_relay.clone()),
                Err(e) => Event::Error(e.to_string()),
            }
        }
        BackEndInput::AddRelayToDb(db_relay) => {
            process_async_with_event(db_add_relay(&pool, db_relay)).await
        }
        BackEndInput::StorePendingEvent(nostr_event) => {
            process_async_with_event(insert_pending_event(
                &pool,
                &keys,
                sender,
                nostr_sender,
                nostr_event,
            ))
            .await
        }
        BackEndInput::StoreEvent((relay_url, nostr_event)) => {
            process_async_with_event(insert_event(
                &pool,
                &keys,
                sender,
                nostr_sender,
                &relay_url,
                nostr_event,
            ))
            .await
        }
        BackEndInput::StoreRelayMessage((relay_url, relay_message)) => {
            process_async_with_event(on_relay_message(&pool, &relay_url, &relay_message)).await
        }
    }
}

async fn handle_profile_picture_update(
    keys: &Keys,
    public_key: XOnlyPublicKey,
    pool: &SqlitePool,
    path: PathBuf,
) -> Event {
    if keys.public_key() == public_key {
        // user
        match UserConfig::update_user_profile_picture(pool, &path).await {
            Ok(_) => Event::UserProfilePictureUpdated,
            Err(e) => Event::Error(e.to_string()),
        }
    } else {
        match DbContact::fetch_one(pool, &public_key).await {
            Ok(Some(mut db_contact)) => {
                db_contact = db_contact.with_local_profile_image(&path);
                match DbContact::update(pool, &db_contact).await {
                    Ok(_) => Event::ContactUpdated(db_contact.clone()),
                    Err(e) => Event::Error(e.to_string()),
                }
            }
            Ok(None) => Event::None,
            Err(e) => Event::Error(e.to_string()),
        }
    }
}

async fn handle_profile_banner_update(
    keys: &Keys,
    public_key: XOnlyPublicKey,
    pool: &SqlitePool,
    path: PathBuf,
) -> Event {
    if keys.public_key() == public_key {
        // user
        match UserConfig::update_user_banner_picture(pool, &path).await {
            Ok(_) => Event::UserBannerPictureUpdated,
            Err(e) => Event::Error(e.to_string()),
        }
    } else {
        match DbContact::fetch_one(pool, &public_key).await {
            Ok(Some(mut db_contact)) => {
                db_contact = db_contact.with_local_banner_image(&path);
                match DbContact::update(pool, &db_contact).await {
                    Ok(_) => Event::ContactUpdated(db_contact.clone()),
                    Err(e) => Event::Error(e.to_string()),
                }
            }
            Ok(None) => Event::None,
            Err(e) => Event::Error(e.to_string()),
        }
    }
}

pub async fn insert_specific_kind(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: Option<&Url>,
    db_event: &DbEvent,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Option<SpecificEvent>, Error> {
    let event = match db_event.kind {
        nostr_sdk::Kind::Metadata => {
            handle_metadata_event(pool, keys, back_sender, db_event).await?
        }
        nostr_sdk::Kind::EncryptedDirectMessage => {
            handle_dm(db_event, relay_url, pool, keys, nostr_sender).await?
        }
        nostr_sdk::Kind::RecommendRelay => handle_recommend_relay(db_event),
        nostr_sdk::Kind::ChannelCreation => {
            // println!("--- ChannelCreation ---");
            // dbg!(db_event);
            None
        }
        nostr_sdk::Kind::ChannelMetadata => {
            // println!("--- ChannelMetadata ---");
            // dbg!(db_event);
            None
        }
        nostr_sdk::Kind::ChannelMessage => {
            // println!("--- ChannelMessage ---");
            // dbg!(db_event);
            None
        }
        nostr_sdk::Kind::ChannelHideMessage => {
            // println!("--- ChannelHideMessage ---");
            // dbg!(db_event);
            None
        }
        nostr_sdk::Kind::ChannelMuteUser => {
            // println!("--- ChannelMuteUser ---");
            // dbg!(db_event);
            None
        }
        // Kind::EventDeletion => todo!(),
        // Kind::PublicChatReserved45 => todo!(),
        // Kind::PublicChatReserved46 => todo!(),
        // Kind::PublicChatReserved47 => todo!(),
        // Kind::PublicChatReserved48 => todo!(),
        // Kind::PublicChatReserved49 => todo!(),
        // Kind::ZapRequest => todo!(),
        // Kind::Zap => todo!(),
        // Kind::MuteList => todo!(),
        // Kind::PinList => todo!(),
        // Kind::RelayList => todo!(),
        // Kind::Authentication => todo!(),
        _other => None,
    };

    Ok(event)
}

async fn handle_contact_list(
    event: nostr_sdk::Event,
    keys: &Keys,
    pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: Option<&Url>,
) -> Result<Event, Error> {
    if event.pubkey == keys.public_key() {
        tracing::info!("Received a ContactList");

        if let Some(last_event) =
            DbEvent::fetch_last_kind(pool, nostr_sdk::Kind::ContactList).await?
        {
            if last_event.created_at_from_relay().timestamp_millis()
                > (event.created_at.as_i64() * 1000)
            {
                tracing::info!("ContactList is older than the last one");
                return Ok(Event::None);
            } else {
                tracing::info!("ContactList is newer than the last one");
                DbEvent::delete(pool, last_event.event_id()?).await?;
            }
        } else {
            tracing::info!("No ContactList in the database");
        }

        // insert event
        tracing::info!("Inserting contact list event");
        tracing::debug!("{:?}", event);
        let mut db_event = DbEvent::confirmed_event(event)?;
        if let Some(url) = relay_url {
            db_event = db_event.with_relay(url);
        }
        let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
        db_event = db_event.with_id(row_id);

        let db_contacts: Vec<_> = db_event
            .tags
            .iter()
            .filter_map(|t| DbContact::from_tag(t).ok())
            .collect();

        // Filter out contacts with the same public key as the user's public key
        let filtered_contacts: Vec<&DbContact> = db_contacts
            .iter()
            .filter(|c| c.pubkey() != &keys.public_key())
            .collect();

        if filtered_contacts.len() < db_contacts.len() {
            tracing::warn!("Error inserting contact: {:?}", Error::SameContactInsert);
        }

        for db_contact in &db_contacts {
            match insert_contact_from_event(
                keys,
                pool,
                &db_event.created_at_from_relay(),
                db_contact,
            )
            .await
            {
                Ok(event) => {
                    if let Err(e) = back_sender.try_send(BackEndInput::Ok(event)) {
                        tracing::error!("Error sending message to backend: {:?}", e);
                    }
                }
                Err(e) => {
                    if let Err(e) = back_sender.try_send(BackEndInput::Error(e.to_string())) {
                        tracing::error!("Error sending message to backend: {:?}", e);
                    }
                }
            }
        }

        if let Err(e) =
            nostr_sender.try_send(NostrInput::GetContactListProfiles(db_contacts.clone()))
        {
            tracing::error!("Error sending message to backend: {:?}", e);
        }

        Ok(Event::EventInserted {
            db_event,
            specific_event: Some(SpecificEvent::ReceivedContactList(db_contacts)),
        })
    } else {
        tracing::info!("*** Others ContactList That Im in ***");
        Ok(Event::None)
    }
}

fn handle_recommend_relay(db_event: &DbEvent) -> Option<SpecificEvent> {
    tracing::info!("--- RecommendRelay ---");
    dbg!(&db_event);
    None
}

async fn handle_dm(
    db_event: &DbEvent,
    relay_url: Option<&Url>,
    pool: &SqlitePool,
    keys: &Keys,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Option<SpecificEvent>, Error> {
    let db_message = DbMessage::from_db_event(db_event, relay_url)?;
    tracing::debug!("Inserting external message");
    // Insert message into the database and get the message ID
    let msg_id = DbMessage::insert_message(pool, &db_message).await?;
    let db_message = db_message.with_id(msg_id);

    // Decrypt the message content
    let content = db_message.decrypt_message(keys)?;

    // Determine if the message is from the user or received from another user
    let (contact_pubkey, is_from_user) = if db_message.im_author(&keys.public_key()) {
        (db_message.to_pubkey(), true)
    } else {
        (db_message.from_pubkey(), false)
    };

    // Fetch the associated contact from the database
    let event = match DbContact::fetch_one(pool, &contact_pubkey).await? {
        Some(mut db_contact) => {
            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;
            SpecificEvent::ReceivedDM((db_contact, chat_message))
        }
        None => {
            // Create a new contact and insert it into the database
            let mut db_contact = DbContact::new(&contact_pubkey);
            insert_contact_from_event(keys, pool, &db_event.created_at_from_relay(), &db_contact)
                .await?;

            // Update last message and contact in the database
            let chat_message =
                ChatMessage::from_db_message(&db_message, is_from_user, &db_contact, &content)?;
            db_contact = DbContact::new_message(pool, &db_contact, &chat_message).await?;

            SpecificEvent::NewDMAndContact((db_contact, chat_message))
        }
    };

    Ok(Some(event))
}
// Handle metadata events and update user profile or contact metadata accordingly.
async fn handle_metadata_event(
    pool: &SqlitePool,
    keys: &Keys,
    sender: &mut mpsc::Sender<BackEndInput>,
    db_event: &DbEvent,
) -> Result<Option<SpecificEvent>, Error> {
    tracing::info!(
        "Received metadata event for public key: {}",
        db_event.pubkey
    );
    tracing::debug!("{:?}", db_event);
    let event_pubkey = &db_event.pubkey;
    let last_update = db_event.created_at_from_relay();
    let metadata = Metadata::from_json(&db_event.content)
        .map_err(|_| Error::JsonToMetadata(db_event.content.to_string()))?;

    let event = if event_pubkey == &keys.public_key() {
        handle_user_metadata_event(pool, &metadata, &last_update).await?
    } else {
        handle_contact_metadata_event(pool, &metadata, event_pubkey, &last_update).await?
    };

    if let Some(_ev) = &event {
        download_images(sender, &metadata, event_pubkey).await?;
    }

    Ok(event)
}

// Handle user metadata events and update user profile metadata if needed.
async fn handle_user_metadata_event(
    pool: &SqlitePool,
    metadata: &Metadata,
    last_update: &NaiveDateTime,
) -> Result<Option<SpecificEvent>, Error> {
    if UserConfig::should_update_user_metadata(pool, last_update).await? {
        UserConfig::update_user_metadata(metadata, last_update, pool).await?;
        Ok(Some(SpecificEvent::UpdatedUserProfileMeta(
            metadata.clone(),
        )))
    } else {
        tracing::warn!("Received outdated metadata for user");
        Ok(None)
    }
}

// Handle contact metadata events and update contact metadata if needed.
async fn handle_contact_metadata_event(
    pool: &SqlitePool,
    metadata: &Metadata,
    pubkey: &XOnlyPublicKey,
    last_update: &NaiveDateTime,
) -> Result<Option<SpecificEvent>, Error> {
    if let Some(mut db_contact) = DbContact::fetch_one(pool, pubkey).await? {
        if should_update_contact_metadata(&db_contact, last_update) {
            db_contact = db_contact.with_profile_meta(metadata, *last_update);
            DbContact::update(pool, &db_contact).await?;
            tracing::info!("Updated contact with profile metadata: {:?}", db_contact);
            Ok(Some(SpecificEvent::UpdatedContactMetadata(db_contact)))
        } else {
            tracing::warn!("Received outdated metadata for contact: {}", pubkey);
            Ok(None)
        }
    } else {
        tracing::warn!("Received metadata for unknown contact: {}", pubkey);
        Ok(None)
    }
}

// Determine if the contact metadata should be updated based on the last update time.
fn should_update_contact_metadata(db_contact: &DbContact, last_update: &NaiveDateTime) -> bool {
    db_contact
        .get_profile_meta_last_update()
        .map(|previous_update| previous_update <= *last_update)
        .unwrap_or(true)
}

async fn download_images(
    sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    public_key: &XOnlyPublicKey,
) -> Result<(), Error> {
    // use a tokio::spawn and channel to download images in parallel
    if let Some(picture_url) = &metadata.picture {
        let mut sender_1 = sender.clone();
        let pic_1 = picture_url.clone();
        let pub_1 = public_key.clone();
        tokio::spawn(async move {
            match download_image(&pic_1, &pub_1, ImageKind::Profile).await {
                Ok(path) => {
                    tracing::info!("Downloaded profile picture for contact: {:?}", &path);
                    if let Err(e) =
                        sender_1.try_send(BackEndInput::ProfilePictureDownloaded((pub_1, path)))
                    {
                        tracing::error!("Error sending message to backend: {:?}", e);
                    }
                }
                Err(e) => {
                    if let Err(e) = sender_1.try_send(BackEndInput::Error(e.to_string())) {
                        tracing::error!("Error sending error to backend: {:?}", e);
                    }
                }
            }
        });
    }
    if let Some(banner_url) = &metadata.banner {
        let mut sender_1 = sender.clone();
        let banner_1 = banner_url.clone();
        let pub_1 = public_key.clone();
        tokio::spawn(async move {
            match download_image(&banner_1, &pub_1, ImageKind::Banner).await {
                Ok(path) => {
                    tracing::info!("Downloaded banner picture for contact: {:?}", &path);
                    if let Err(e) =
                        sender_1.try_send(BackEndInput::ProfileBannerDownloaded((pub_1, path)))
                    {
                        tracing::error!("Error sending message to backend: {:?}", e);
                    }
                }
                Err(e) => {
                    if let Err(e) = sender_1.try_send(BackEndInput::Error(e.to_string())) {
                        tracing::error!("Error sending error to backend: {:?}", e);
                    }
                }
            }
        });
    }
    Ok(())
}

pub async fn handle_insert_event(
    pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    event: nostr_sdk::Event,
    relay_url: Option<&Url>,
    is_pending: bool,
) -> Result<Event, Error> {
    tracing::info!(
        "Inserting [{}] event",
        if is_pending { "pending" } else { "confirmed" },
    );
    tracing::debug!("{:?}", event);

    let mut db_event = if is_pending {
        DbEvent::pending_event(event)?
    } else {
        DbEvent::confirmed_event(event)?
    };
    if let Some(url) = relay_url {
        db_event = db_event.with_relay(url);
    }

    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    // after db_event is inserted, we can update the relay_response
    if let Some(url) = relay_url {
        let _ev = relay_response_ok(pool, &db_event, url).await?;
    }

    if rows_changed == 0 {
        return Ok(Event::None);
    }

    let specific_event =
        insert_specific_kind(pool, keys, relay_url, &db_event, back_sender, nostr_sender).await?;

    if is_pending {
        Ok(Event::LocalPendingEvent {
            db_event,
            specific_event,
        })
    } else {
        Ok(Event::EventInserted {
            db_event,
            specific_event,
        })
    }
}

pub async fn insert_event(
    pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
    event: nostr_sdk::Event,
) -> Result<Event, Error> {
    match event.kind {
        nostr_sdk::Kind::ContactList => {
            handle_contact_list(
                event,
                keys,
                pool,
                back_sender,
                nostr_sender,
                Some(relay_url),
            )
            .await
        }
        _other => {
            handle_insert_event(
                pool,
                keys,
                back_sender,
                nostr_sender,
                event,
                Some(relay_url),
                false,
            )
            .await
        }
    }
}

pub async fn insert_pending_event(
    pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    event: nostr_sdk::Event,
) -> Result<Event, Error> {
    handle_insert_event(pool, keys, back_sender, nostr_sender, event, None, true).await
}

pub async fn relay_response_ok(
    pool: &SqlitePool,
    db_event: &DbEvent,
    relay_url: &Url,
) -> Result<Event, Error> {
    let mut relay_response = DbRelayResponse::from_response(
        true,
        db_event.event_id()?,
        &db_event.event_hash,
        relay_url,
        "",
    );
    let id = DbRelayResponse::insert(pool, &relay_response).await?;
    relay_response = relay_response.with_id(id);
    Ok(Event::UpdateWithRelayResponse {
        relay_response,
        db_event: db_event.clone(),
        db_message: None,
    })
}

pub async fn insert_contact_from_event(
    keys: &Keys,
    pool: &SqlitePool,
    event_date: &NaiveDateTime,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactInsert);
    }

    // Check if the contact is already in the database
    if DbContact::have_contact(pool, &db_contact.pubkey()).await? {
        return update_contact_basic(keys, pool, db_contact).await;
    }

    // If the contact is not in the database, insert it
    DbContact::insert(pool, &db_contact).await?;

    Ok(Event::ContactCreated(db_contact.clone()))
}

pub async fn update_contact_basic(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactUpdate);
    }
    DbContact::update_basic(pool, &db_contact).await?;
    Ok(Event::ContactUpdated(db_contact.clone()))
}

pub async fn update_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactUpdate);
    }
    DbContact::update(pool, &db_contact).await?;
    Ok(Event::ContactUpdated(db_contact.clone()))
}

pub async fn add_new_contact(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        return Err(Error::SameContactInsert);
    }

    DbContact::insert(pool, &db_contact).await?;

    Ok(Event::ContactCreated(db_contact.clone()))
}

pub async fn delete_contact(pool: &SqlitePool, db_contact: &DbContact) -> Result<Event, Error> {
    DbContact::delete(pool, &db_contact).await?;
    Ok(Event::ContactDeleted(db_contact.clone()))
}

pub async fn prepare_client(pool: &SqlitePool) -> Result<NostrInput, Error> {
    tracing::info!("Preparing client");
    let relays = DbRelay::fetch(pool, None).await?;
    let last_event = DbEvent::fetch_last(pool).await?;

    Ok(NostrInput::PrepareClient { relays, last_event })
}

pub async fn on_relay_message(
    pool: &SqlitePool,
    relay_url: &Url,
    relay_message: &RelayMessage,
) -> Result<Event, Error> {
    tracing::info!("New relay message: {}", relay_url);
    tracing::debug!("{:?}", relay_message);
    let event = match relay_message {
        RelayMessage::Ok {
            event_id: event_hash,
            status,
            message,
        } => {
            tracing::info!("Relay message: Ok");
            let mut db_event = DbEvent::fetch_one(pool, event_hash)
                .await?
                .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;
            let mut db_message = None;

            if !db_event.confirmed {
                db_event = DbEvent::confirm_event(pool, relay_url, db_event).await?;

                if let nostr_sdk::Kind::EncryptedDirectMessage = db_event.kind {
                    db_message = if let Some(db_message) =
                        DbMessage::fetch_one(pool, db_event.event_id()?).await?
                    {
                        let confirmed_db_message =
                            DbMessage::confirm_message(pool, db_message).await?;
                        Some(confirmed_db_message)
                    } else {
                        None
                    };
                }
            }

            let mut relay_response = DbRelayResponse::from_response(
                *status,
                db_event.event_id()?,
                event_hash,
                relay_url,
                message,
            );
            let id = DbRelayResponse::insert(pool, &relay_response).await?;
            relay_response = relay_response.with_id(id);
            Event::UpdateWithRelayResponse {
                relay_response,
                db_event,
                db_message,
            }
        }
        RelayMessage::EndOfStoredEvents(subscription_id) => {
            tracing::info!("Relay message: EOSE. ID: {}", subscription_id);
            Event::EndOfStoredEvents((relay_url.to_owned(), subscription_id.to_owned()))
        }
        RelayMessage::Event {
            subscription_id,
            event,
        } => {
            tracing::debug!("Relay message: Event. ID: {}", subscription_id);
            tracing::debug!("{:?}", event);
            Event::None
        }
        RelayMessage::Notice { message } => {
            tracing::info!("Relay message: Notice: {}", message);
            Event::None
        }
        RelayMessage::Auth { challenge } => {
            tracing::info!("Relay message: Auth Challenge: {}", challenge);
            Event::None
        }
        RelayMessage::Count {
            subscription_id: _,
            count,
        } => {
            tracing::info!("Relay message: Count: {}", count);
            Event::None
        }
        RelayMessage::Empty => {
            tracing::info!("Relay message: Empty");
            Event::None
        }
    };

    Ok(event)
}

pub async fn add_to_unseen_count(
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<Event, Error> {
    db_contact = DbContact::add_to_unseen_count(pool, &mut db_contact).await?;
    Ok(Event::ContactUpdated(db_contact))
}

pub async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<Event, Error> {
    tracing::info!("Fetching chat messages");
    let own_pubkey = keys.public_key();
    let chat = DbChat::new(&own_pubkey, db_contact.pubkey());
    let mut db_messages = chat.fetch_chat(pool).await?;
    let mut chat_messages = vec![];

    tracing::info!("Updating unseen messages to marked as seen");
    for m in db_messages.iter_mut().filter(|m| m.is_unseen()) {
        m.update_status(MessageStatus::Seen);
        DbMessage::update_message_status(pool, m).await?;
    }

    tracing::info!("Decrypting messages");
    for m in &mut db_messages {
        let content = m.decrypt_message(keys)?;
        let is_from_user = m.im_author(&keys.public_key());
        let chat_message = ChatMessage::from_db_message(&m, is_from_user, &db_contact, &content)?;
        chat_messages.push(chat_message);
    }

    db_contact = DbContact::update_unseen_count(pool, &mut db_contact, 0).await?;

    Ok(Event::GotChatMessages((db_contact, chat_messages)))
}

pub async fn fetch_relays_responses(pool: &SqlitePool, event_id: i64) -> Result<Event, Error> {
    let responses = DbRelayResponse::fetch_by_event(pool, event_id).await?;
    Ok(Event::GotRelayResponses(responses))
}

pub async fn db_add_relay(pool: &SqlitePool, db_relay: DbRelay) -> Result<Event, Error> {
    DbRelay::insert(pool, &db_relay).await?;
    Ok(Event::RelayCreated(db_relay))
}

pub async fn fetch_contacts(pool: &SqlitePool) -> Result<Event, Error> {
    let contacts = DbContact::fetch(pool).await?;
    Ok(Event::GotContacts(contacts))
}

pub async fn db_delete_relay(pool: &SqlitePool, db_relay: DbRelay) -> Result<Event, Error> {
    DbRelay::delete(pool, &db_relay).await?;
    Ok(Event::RelayDeleted(db_relay))
}
pub async fn fetch_relays(pool: &SqlitePool) -> Result<Event, Error> {
    let relays = DbRelay::fetch(pool, None).await?;
    Ok(Event::GotRelays(relays))
}
