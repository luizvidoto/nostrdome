use std::path::PathBuf;

use chrono::NaiveDateTime;
use futures::channel::mpsc;
use nostr_sdk::{secp256k1::XOnlyPublicKey, Keys, Metadata, RelayMessage, Url};
use sqlx::SqlitePool;

use crate::{
    db::{DbContact, DbEvent, DbMessage, DbRelay, DbRelayResponse, TagInfo, UserConfig},
    error::Error,
    net::{
        client::{download_image, ImageKind},
        events::nostr::NostrInput,
        process_async_with_event,
    },
    ntp::ntp_request,
    types::ChatMessage,
};

use super::Event;

#[derive(Debug, Clone)]
pub enum BackEndInput {
    NtpTime(u64),
    ToggleRelayRead((DbRelay, bool)),
    ToggleRelayWrite((DbRelay, bool)),
    AddRelayToDb(DbRelay),
    DeleteRelayFromDb(DbRelay),
    StorePendingContactList((nostr_sdk::Event, Vec<DbContact>)),
    StorePendingMetadata((nostr_sdk::Event, nostr_sdk::Metadata)),
    StorePendingMessage {
        ns_event: nostr_sdk::Event,
        db_contact: DbContact,
        content: String,
    },
    StoreConfirmedEvent((nostr_sdk::Url, nostr_sdk::Event)),
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
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Event {
    match input {
        BackEndInput::Ok(event) => event,
        BackEndInput::Error(e) => Event::Error(e),
        // --- REQWEST ---
        BackEndInput::LatestVersion(version) => Event::LatestVersion(version),
        BackEndInput::Shutdown => Event::None,
        // --- TO DATABASE ---
        BackEndInput::NtpTime(total_microseconds) => {
            tracing::info!("NTP time: {}", total_microseconds);
            match UserConfig::update_ntp_offset(pool, total_microseconds).await {
                Ok(_) => Event::SyncedWithNtpServer,
                Err(e) => Event::Error(e.to_string()),
            }
        }
        BackEndInput::FinishedPreparingNostr => {
            // donwload images?
            if let Ok(contact_list) = DbContact::fetch(pool).await {
                for c in contact_list {
                    if let Some(metadata) = c.get_profile_meta() {
                        download_profile_image(back_sender, &metadata, c.pubkey()).await
                    }
                }
            }
            Event::FinishedPreparing
        }
        BackEndInput::ProfilePictureDownloaded((public_key, path)) => {
            handle_profile_picture_update(keys, public_key, pool, path).await
        }
        BackEndInput::ProfileBannerDownloaded((public_key, path)) => {
            handle_profile_banner_update(keys, public_key, pool, path).await
        }
        BackEndInput::DeleteRelayFromDb(db_relay) => match DbRelay::delete(pool, &db_relay).await {
            Ok(_) => Event::RelayDeleted(db_relay),
            Err(e) => Event::Error(e.to_string()),
        },
        BackEndInput::StorePendingMessage {
            ns_event,
            db_contact,
            content,
        } => {
            process_async_with_event(insert_pending_dm(
                pool,
                keys,
                ns_event,
                db_contact,
                content,
                nostr_sender,
            ))
            .await
        }
        BackEndInput::StorePendingContactList((ns_event, contact_list)) => {
            process_async_with_event(insert_pending_contact_list(
                pool,
                ns_event,
                contact_list,
                nostr_sender,
            ))
            .await
        }
        BackEndInput::StorePendingMetadata((ns_event, metadata)) => {
            process_async_with_event(insert_pending_metadata(
                pool,
                ns_event,
                metadata,
                nostr_sender,
            ))
            .await
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
        BackEndInput::AddRelayToDb(db_relay) => match DbRelay::insert(pool, &db_relay).await {
            Ok(_) => Event::RelayCreated(db_relay),
            Err(e) => Event::Error(e.to_string()),
        },
        BackEndInput::StoreConfirmedEvent((relay_url, ns_event)) => {
            process_async_with_event(insert_confirmed_event(
                pool,
                keys,
                back_sender,
                nostr_sender,
                &relay_url,
                ns_event,
            ))
            .await
        }
        BackEndInput::StoreRelayMessage((relay_url, relay_message)) => {
            process_async_with_event(on_relay_message(&pool, keys, &relay_url, &relay_message))
                .await
        }
    }
}

async fn handle_profile_picture_update(
    keys: &Keys,
    public_key: XOnlyPublicKey,
    pool: &SqlitePool,
    path: PathBuf,
) -> Event {
    tracing::debug!("handle_profile_picture_update");
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
    tracing::debug!("handle_profile_banner_update");
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

pub async fn on_relay_message(
    pool: &SqlitePool,
    keys: &Keys,
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
            let (db_event, db_message, db_contact) =
                confirm_event_and_message(pool, keys, event_hash, relay_url).await?;

            let mut relay_response = DbRelayResponse::from_response(
                *status,
                db_event.event_id()?,
                event_hash,
                relay_url,
                message,
            );
            let id = DbRelayResponse::insert(pool, &relay_response).await?;
            relay_response = relay_response.with_id(id);
            Event::RelayConfirmation {
                relay_response,
                db_event,
                db_message,
                db_contact,
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
            tracing::warn!("Relay message: Auth Challenge: {}", challenge);
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

async fn last_kind_filtered(pool: &SqlitePool) -> Result<Option<(NaiveDateTime, i64)>, Error> {
    let last_event = match DbEvent::fetch_last_kind(pool, nostr_sdk::Kind::ContactList).await? {
        Some(last_event) => last_event,
        None => return Ok(None),
    };

    match (last_event.remote_creation(), last_event.event_id()) {
        (Some(remote_creation), Ok(event_id)) => Ok(Some((remote_creation, event_id))),
        _ => Ok(None),
    }
}
async fn handle_contact_list(
    event: nostr_sdk::Event,
    keys: &Keys,
    pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
) -> Result<Event, Error> {
    if event.pubkey == keys.public_key() {
        handle_user_contact_list(event, keys, pool, back_sender, nostr_sender, relay_url).await
    } else {
        handle_other_contact_list(event)
    }
}

async fn handle_user_contact_list(
    event: nostr_sdk::Event,
    keys: &Keys,
    pool: &SqlitePool,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
) -> Result<Event, Error> {
    tracing::debug!("Received a ContactList");

    if let Some((remote_creation, event_id)) = last_kind_filtered(pool).await? {
        if remote_creation.timestamp_millis() > (event.created_at.as_i64() * 1000) {
            tracing::info!("ContactList is older than the last one");
            return Ok(Event::None);
        } else {
            tracing::info!("ContactList is newer than the last one");
            DbEvent::delete(pool, event_id).await?;
        }
    } else {
        tracing::info!("No ContactList in the database");
    }

    tracing::info!("Inserting contact list event");
    tracing::debug!("{:?}", event);
    // create event struct
    let mut db_event = DbEvent::confirmed_event(event, relay_url)?;
    // insert into database
    let (row_id, _rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    // contact list from event tags
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
        match insert_contact_from_event(keys, pool, db_contact).await {
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

    if let Err(e) = nostr_sender.try_send(NostrInput::GetContactListProfiles(db_contacts.clone())) {
        tracing::error!("Error sending message to backend: {:?}", e);
    }

    Ok(Event::ReceivedContactList {
        contact_list: db_contacts,
        relay_url: relay_url.to_owned(),
    })
}

fn handle_other_contact_list(_event: nostr_sdk::Event) -> Result<Event, Error> {
    tracing::info!("*** Others ContactList That Im in ***");
    Ok(Event::None)
}

async fn handle_recommend_relay(ns_event: nostr_sdk::Event) -> Result<Event, Error> {
    tracing::debug!("handle_recommend_relay");
    dbg!(&ns_event);
    Ok(Event::None)
}

async fn handle_dm(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("handle_dm");
    // create event struct
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    // insert into database
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    if rows_changed == 0 {
        tracing::info!("Event already in database");
        return Ok(Event::None);
    }

    let info = TagInfo::from_db_event(&db_event)?;
    let contact_pubkey = info.contact_pubkey(keys)?;

    // Parse the event and insert the message into the database
    let db_message = insert_message_from_event(&db_event, pool, &contact_pubkey).await?;

    // If the message is from the user to themselves, log the error and return None
    if db_message.from_pubkey() == db_message.to_pubkey() {
        tracing::error!("Message is from the user to himself");
        return Ok(Event::None);
    }

    // Determine the contact's public key and whether the message is from the user
    let (contact_pubkey, _is_from_user) =
        determine_sender_receiver(&db_message, &keys.public_key());

    // Fetch or create the associated contact, update the contact's message, and return the event
    let event =
        fetch_or_create_contact(pool, keys, relay_url, &contact_pubkey, &db_message).await?;

    Ok(event)
}

async fn insert_message_from_event(
    db_event: &DbEvent,
    pool: &SqlitePool,
    contact_pubkey: &XOnlyPublicKey,
) -> Result<DbMessage, Error> {
    tracing::debug!("insert_message_from_event");
    let db_message = DbMessage::confirmed_message(db_event, contact_pubkey)?;
    let msg_id = DbMessage::insert_message(pool, &db_message).await?;
    Ok(db_message.with_id(msg_id))
}

fn determine_sender_receiver(
    db_message: &DbMessage,
    user_pubkey: &XOnlyPublicKey,
) -> (XOnlyPublicKey, bool) {
    tracing::debug!("determine_sender_receiver");
    if db_message.im_author(user_pubkey) {
        tracing::info!("Message is from the user");
        (db_message.to_pubkey(), true)
    } else {
        tracing::info!("Message is from contact");
        (db_message.from_pubkey(), false)
    }
}

async fn fetch_or_create_contact(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    contact_pubkey: &XOnlyPublicKey,
    db_message: &DbMessage,
) -> Result<Event, Error> {
    tracing::debug!("fetch_or_create_contact");
    let mut db_contact = match DbContact::fetch_one(pool, contact_pubkey).await? {
        Some(db_contact) => db_contact,
        None => {
            tracing::info!("Creating new contact with pubkey: {}", contact_pubkey);
            let db_contact = DbContact::new(contact_pubkey);
            DbContact::insert(pool, &db_contact).await?;
            db_contact
        }
    };

    tracing::debug!("Update last message and contact in the database");
    let chat_message = ChatMessage::from_db_message(keys, db_message, &db_contact)?;
    db_contact = DbContact::new_message(pool, db_contact, &chat_message).await?;
    Ok(Event::ReceivedDM {
        chat_message,
        db_contact,
        relay_url: relay_url.to_owned(),
    })
}

// Handle metadata events and update user profile or contact metadata accordingly.
async fn handle_metadata_event(
    pool: &SqlitePool,
    keys: &Keys,
    relay_url: &Url,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("handle_metadata_event");

    // create event struct
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    // insert into database
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);

    if rows_changed == 0 {
        tracing::info!("Event already in database");
        return Ok(Event::None);
    }

    tracing::info!(
        "Received metadata event for public key: {}",
        db_event.pubkey
    );
    tracing::debug!("{:?}", db_event);
    let event_pubkey = &db_event.pubkey;
    let last_update = db_event.remote_creation().ok_or(Error::NotConfirmedEvent)?;
    let metadata = Metadata::from_json(&db_event.content)
        .map_err(|_| Error::JsonToMetadata(db_event.content.to_string()))?;

    let event = if event_pubkey == &keys.public_key() {
        handle_user_metadata_event(pool, relay_url, &metadata, &last_update).await?
    } else {
        handle_contact_metadata_event(pool, relay_url, &metadata, event_pubkey, &last_update)
            .await?
    };

    Ok(event)
}

// Handle user metadata events and update user profile metadata if needed.
async fn handle_user_metadata_event(
    pool: &SqlitePool,
    relay_url: &Url,
    metadata: &Metadata,
    last_update: &NaiveDateTime,
) -> Result<Event, Error> {
    tracing::debug!("handle_user_metadata_event");
    if UserConfig::should_update_user_metadata(pool, last_update).await? {
        UserConfig::update_user_metadata(metadata, last_update, pool).await?;
        Ok(Event::UpdatedUserProfileMeta {
            relay_url: relay_url.clone(),
            metadata: metadata.clone(),
        })
    } else {
        tracing::warn!("Received outdated metadata for user");
        Ok(Event::None)
    }
}

// Handle contact metadata events and update contact metadata if needed.
async fn handle_contact_metadata_event(
    pool: &SqlitePool,
    relay_url: &Url,
    metadata: &Metadata,
    pubkey: &XOnlyPublicKey,
    last_update: &NaiveDateTime,
) -> Result<Event, Error> {
    tracing::debug!("handle_contact_metadata_event");
    if let Some(mut db_contact) = DbContact::fetch_one(pool, pubkey).await? {
        if should_update_contact_metadata(&db_contact, last_update) {
            db_contact = db_contact.with_profile_meta(metadata, *last_update);
            DbContact::update(pool, &db_contact).await?;
            tracing::info!("Updated contact with profile metadata");
            tracing::debug!("{:?}", db_contact);
            Ok(Event::UpdatedContactMetadata {
                db_contact,
                relay_url: relay_url.clone(),
            })
        } else {
            tracing::warn!("Received outdated metadata for contact: {}", pubkey);
            Ok(Event::None)
        }
    } else {
        tracing::warn!("Received metadata for unknown contact: {}", pubkey);
        Ok(Event::None)
    }
}

// Determine if the contact metadata should be updated based on the last update time.
fn should_update_contact_metadata(db_contact: &DbContact, last_update: &NaiveDateTime) -> bool {
    db_contact
        .get_profile_meta_last_update()
        .map(|previous_update| previous_update <= *last_update)
        .unwrap_or(true)
}

async fn download_profile_image(
    back_sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    public_key: &XOnlyPublicKey,
) {
    if let Some(picture_url) = &metadata.picture {
        let mut sender_1 = back_sender.clone();
        let pic_1 = picture_url.clone();
        let pub_1 = public_key.clone();
        tokio::spawn(async move {
            match download_image(&pic_1, &pub_1, ImageKind::Profile).await {
                Ok(path) => {
                    tracing::info!("Downloaded profile picture for pubkey: {:?}", &path);
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
    } else {
        tracing::warn!("No profile picture to download");
    }
}

async fn download_banner_image(
    sender: &mut mpsc::Sender<BackEndInput>,
    metadata: &Metadata,
    public_key: &XOnlyPublicKey,
) -> Result<(), Error> {
    if let Some(banner_url) = &metadata.banner {
        let mut sender_1 = sender.clone();
        let banner_1 = banner_url.clone();
        let pub_1 = public_key.clone();
        tokio::spawn(async move {
            match download_image(&banner_1, &pub_1, ImageKind::Banner).await {
                Ok(path) => {
                    tracing::info!("Downloaded banner picture for pubkey: {:?}", &path);
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
    } else {
        tracing::warn!("No banner image to download");
    }
    Ok(())
}

pub async fn insert_confirmed_event(
    pool: &SqlitePool,
    keys: &Keys,
    back_sender: &mut mpsc::Sender<BackEndInput>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
    relay_url: &Url,
    ns_event: nostr_sdk::Event,
) -> Result<Event, Error> {
    tracing::debug!("insert_event");
    match ns_event.kind {
        nostr_sdk::Kind::ContactList => {
            handle_contact_list(ns_event, keys, pool, back_sender, nostr_sender, relay_url).await
        }
        nostr_sdk::Kind::Metadata => handle_metadata_event(pool, keys, relay_url, ns_event).await,
        nostr_sdk::Kind::EncryptedDirectMessage => handle_dm(pool, keys, relay_url, ns_event).await,
        nostr_sdk::Kind::RecommendRelay => handle_recommend_relay(ns_event).await,
        // nostr_sdk::Kind::ChannelCreation => {}
        // nostr_sdk::Kind::ChannelMetadata => {
        // nostr_sdk::Kind::ChannelMessage => {}
        // nostr_sdk::Kind::ChannelHideMessage => {}
        // nostr_sdk::Kind::ChannelMuteUser => {}
        // Kind::EventDeletion => {},
        // Kind::PublicChatReserved45 => {},
        // Kind::PublicChatReserved46 => {},
        // Kind::PublicChatReserved47 => {},
        // Kind::PublicChatReserved48 => {},
        // Kind::PublicChatReserved49 => {},
        // Kind::ZapRequest => {},
        // Kind::Zap => {},
        // Kind::MuteList => {},
        // Kind::PinList => {},
        // Kind::RelayList => {},
        // Kind::Authentication => {},
        _other_kind => insert_other_kind(ns_event, relay_url, pool).await,
    }
}

async fn insert_pending_metadata(
    pool: &SqlitePool,
    ns_event: nostr_sdk::Event,
    _metadata: nostr_sdk::Metadata,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    let mut pending_event = DbEvent::pending_event(ns_event.clone())?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &pending_event).await?;
    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Ok(Event::None);
    }
    pending_event = pending_event.with_id(row_id);
    if let Err(e) = nostr_sender.try_send(NostrInput::SendEventToRelays(ns_event)) {
        tracing::error!("Error sending message to nostr: {:?}", e);
    }
    Ok(Event::PendingMetadata(pending_event))
}

async fn insert_pending_contact_list(
    pool: &SqlitePool,
    ns_event: nostr_sdk::Event,
    _contact_list: Vec<DbContact>,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    let mut pending_event = DbEvent::pending_event(ns_event.clone())?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &pending_event).await?;
    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Ok(Event::None);
    }
    pending_event = pending_event.with_id(row_id);
    if let Err(e) = nostr_sender.try_send(NostrInput::SendEventToRelays(ns_event)) {
        tracing::error!("Error sending message to nostr: {:?}", e);
    }
    Ok(Event::PendingContactList(pending_event))
}

async fn insert_pending_dm(
    pool: &SqlitePool,
    keys: &Keys,
    ns_event: nostr_sdk::Event,
    db_contact: DbContact,
    content: String,
    nostr_sender: &mut mpsc::Sender<NostrInput>,
) -> Result<Event, Error> {
    let pending_event = DbEvent::pending_event(ns_event.clone())?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &pending_event).await?;

    if rows_changed == 0 {
        tracing::warn!(
            "Received duplicate pending event: {:?}",
            pending_event.event_hash
        );
        return Ok(Event::None);
    }
    let pending_event = pending_event.with_id(row_id);

    if let Err(e) = nostr_sender.try_send(NostrInput::SendEventToRelays(ns_event)) {
        tracing::error!("Error sending message to nostr: {:?}", e);
    }

    let pending_msg = DbMessage::new(&pending_event, db_contact.pubkey())?;
    let row_id = DbMessage::insert_message(pool, &pending_msg).await?;
    let pending_msg = pending_msg.with_id(row_id);

    let chat_message =
        ChatMessage::from_db_message_content(keys, &pending_msg, &db_contact, &content)?;
    // let db_contact = DbContact::new_message(pool, db_contact, &chat_message).await?;
    Ok(Event::PendingDM((db_contact, chat_message)))
}

async fn insert_other_kind(
    ns_event: nostr_sdk::Event,
    relay_url: &nostr_sdk::Url,
    pool: &SqlitePool,
) -> Result<Event, Error> {
    let mut db_event = DbEvent::confirmed_event(ns_event, relay_url)?;
    let (row_id, rows_changed) = DbEvent::insert(pool, &db_event).await?;
    db_event = db_event.with_id(row_id);
    let _ev = relay_response_ok(pool, &db_event, relay_url).await?;
    if rows_changed == 0 {
        tracing::info!("Received duplicate event: {:?}", db_event.event_hash);
        return Ok(Event::None);
    }
    Ok(Event::OtherKindEventInserted(db_event))
}

pub async fn relay_response_ok(
    pool: &SqlitePool,
    db_event: &DbEvent,
    relay_url: &Url,
) -> Result<Event, Error> {
    tracing::info!("Updating relay response");
    let mut relay_response = DbRelayResponse::from_response(
        true,
        db_event.event_id()?,
        &db_event.event_hash,
        relay_url,
        "",
    );
    let id = DbRelayResponse::insert(pool, &relay_response).await?;
    relay_response = relay_response.with_id(id);
    // update db_message ?
    Ok(Event::RelayConfirmation {
        relay_response,
        db_event: db_event.clone(),
        db_message: None,
        db_contact: None,
    })
}

pub async fn insert_contact_from_event(
    keys: &Keys,
    pool: &SqlitePool,
    db_contact: &DbContact,
) -> Result<Event, Error> {
    tracing::debug!("Inserting contact from event");
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        tracing::warn!("{:?}", Error::SameContactInsert);
        return Ok(Event::None);
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
    tracing::debug!("update_contact_basic");
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
    tracing::debug!("update_contact");
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
) -> Result<Event, Error> {
    tracing::debug!("add_new_contact");
    // Check if the contact is the same as the user
    if &keys.public_key() == db_contact.pubkey() {
        tracing::warn!("{:?}", Error::SameContactInsert);
        return Ok(Event::None);
    }

    DbContact::insert(pool, &db_contact).await?;

    Ok(Event::ContactCreated(db_contact.clone()))
}

pub async fn delete_contact(pool: &SqlitePool, db_contact: &DbContact) -> Result<Event, Error> {
    tracing::debug!("delete_contact");
    DbContact::delete(pool, &db_contact).await?;
    Ok(Event::ContactDeleted(db_contact.clone()))
}

pub async fn prepare_client(pool: &SqlitePool) -> Result<NostrInput, Error> {
    tracing::debug!("prepare_client");
    tracing::info!("Fetching relays and last event to nostr client");
    let relays = DbRelay::fetch(pool).await?;
    let last_event = DbEvent::fetch_last(pool).await?;
    let contact_list = DbContact::fetch(pool).await?;

    Ok(NostrInput::PrepareClient {
        relays,
        last_event,
        contact_list,
    })
}

async fn confirm_event_and_message(
    pool: &SqlitePool,
    keys: &Keys,
    event_hash: &nostr_sdk::EventId,
    relay_url: &Url,
) -> Result<(DbEvent, Option<DbMessage>, Option<DbContact>), Error> {
    let mut db_event = DbEvent::fetch_one(pool, event_hash)
        .await?
        .ok_or(Error::EventNotInDatabase(event_hash.to_owned()))?;
    let mut pair = (None, None);
    if db_event.relay_url.is_none() {
        db_event = DbEvent::confirm_event(pool, relay_url, db_event).await?;

        if let nostr_sdk::Kind::EncryptedDirectMessage = db_event.kind {
            pair =
                if let Some(db_message) = DbMessage::fetch_one(pool, db_event.event_id()?).await? {
                    let confirmed_db_message =
                        DbMessage::relay_confirmation(pool, relay_url, db_message).await?;
                    // add relay confirmation

                    if let Some(db_contact) =
                        DbContact::fetch_one(pool, &confirmed_db_message.contact_chat()).await?
                    {
                        let chat_message =
                            ChatMessage::from_db_message(keys, &confirmed_db_message, &db_contact)?;
                        let db_contact =
                            DbContact::new_message(pool, db_contact, &chat_message).await?;
                        (Some(confirmed_db_message), Some(db_contact))
                    } else {
                        (Some(confirmed_db_message), None)
                    }
                } else {
                    (None, None)
                };
        }
    }
    Ok((db_event, pair.0, pair.1))
}

pub async fn fetch_and_decrypt_chat(
    keys: &Keys,
    pool: &SqlitePool,
    mut db_contact: DbContact,
) -> Result<Event, Error> {
    tracing::info!("Fetching chat messages");
    let mut db_messages = DbMessage::fetch_chat(pool, db_contact.pubkey()).await?;
    let mut chat_messages = vec![];

    tracing::info!("Updating unseen messages to marked as seen");
    for db_message in db_messages.iter_mut().filter(|m| m.is_unseen()) {
        DbMessage::message_seen(pool, db_message).await?;
    }

    tracing::info!("Decrypting messages");
    for db_message in &db_messages {
        let chat_message = ChatMessage::from_db_message(keys, &db_message, &db_contact)?;
        chat_messages.push(chat_message);
    }

    db_contact = DbContact::update_unseen_count(pool, &mut db_contact, 0).await?;

    Ok(Event::GotChatMessages((db_contact, chat_messages)))
}

// pub async fn fetch_relays_responses(pool: &SqlitePool, event_id: i64) -> Result<Event, Error> {
//     tracing::debug!("Fetching relay responses for event: {}", event_id);
//     let responses = DbRelayResponse::fetch_by_event(pool, event_id).await?;
//     Ok(Event::GotRelayResponses(responses))
// }

// pub async fn fetch_contacts(pool: &SqlitePool) -> Result<Event, Error> {
//     tracing::debug!("fetch_contacts");
//     let contacts = DbContact::fetch(pool).await?;
//     Ok(Event::GotContacts(contacts))
// }
