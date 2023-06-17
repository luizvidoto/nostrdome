use futures::channel::mpsc::Sender;

pub struct DmData {
    output: Sender<BackendEvent>,
    pool: SqlitePool,
    cache_pool: SqlitePool,
    keys: Keys,
    url: Url,
}

impl DmData {
    async fn process_and_generate_message(
        &self,
        event_pubkey: &str,
        event_hash: &str,
        tags: &[String],
        db_event_option: Option<DbEvent>,
    ) -> Result<ChatMessage, MyError> {
        let tag_info = process_dm(
            event_pubkey,
            &self.keys,
            MessageTagInfo::from_event_tags(event_hash, event_pubkey, tags),
            "Message from anyone to unknown chat, ignoring",
        )?;

        let db_event = db_event_option
            .or_else(|| DbEvent::insert(&self.pool, &self.url, &ns_event).await?)
            .ok_or(MyError::EventHandlingError)?;

        let chat_pubkey = tag_info
            .chat_pubkey(&self.keys)
            .ok_or(MyError::PubkeyRetrievalError)?;

        self.generate_chat_message(tag_info, db_event, chat_pubkey)
            .await
    }

    async fn generate_chat_message(
        &self,
        tag_info: MessageTagInfo,
        db_event: DbEvent,
        chat_pubkey: String,
    ) -> Result<ChatMessage, MyError> {
        let db_message =
            DbMessage::insert_confirmed(&self.pool, &db_event, &chat_pubkey, is_users).await?;
        let db_contact =
            DbContact::fetch_insert(&self.pool, &self.cache_pool, &db_message.chat_pubkey).await?;
        let decrypted_content = db_message.decrypt_message(&self.keys, &tag_info)?;
        Ok(ChatMessage::new(
            &db_message,
            &db_event.pubkey,
            &db_contact,
            &decrypted_content,
        ))
    }

    pub async fn handle_dm(&self, ns_event: nostr::Event) -> Result<(), MyError> {
        let is_users = ns_event.pubkey == self.keys.public_key();
        let chat_message = self
            .process_and_generate_message(&ns_event.pubkey, &ns_event.id, &ns_event.tags, None)
            .await?;

        let _ = self
            .output
            .send(BackendEvent::ReceivedDM {
                chat_message: chat_message.clone(),
                db_contact: chat_message.contact.clone(),
                relay_url: self.url.to_owned(),
            })
            .await;

        Ok(())
    }

    pub async fn pending_dm_confirmed(&self, db_event: &DbEvent) -> Result<(), MyError> {
        let is_users = db_event.pubkey == self.keys.public_key();
        let chat_message = self
            .process_and_generate_message(
                &db_event.pubkey,
                &db_event.event_hash,
                &db_event.tags,
                Some(db_event.clone()),
            )
            .await?;

        let _ = self
            .output
            .send(BackendEvent::ConfirmedDM(chat_message))
            .await;

        Ok(())
    }
}

fn process_dm<'a>(
    event_pubkey: &'a str,
    keys: &Keys,
    tag_info: Result<MessageTagInfo, MyError>,
    debug_message: &'a str,
) -> Result<MessageTagInfo, MyError> {
    let tag_info = match tag_info {
        Ok(info) => info,
        Err(_) => {
            tracing::debug!(debug_message);
            return Err(MyError::TagInfoProcessingError);
        }
    };

    if tag_info.to_pubkey == event_pubkey {
        tracing::debug!("Message is from the user to himself");
        Err(MyError::SelfMessageError)
    } else {
        Ok(tag_info)
    }
}
