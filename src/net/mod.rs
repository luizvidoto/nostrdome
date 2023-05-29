use crate::db::Database;
use crate::net::backend::{backend_processing, process_message, BackEndInput};
use crate::net::ntp::spawn_ntp_request;

use futures::channel::mpsc;
use futures::StreamExt;
use iced::subscription;
use iced::Subscription;
use nostr::Keys;

mod backend;
mod nostr_events;
pub(crate) mod ntp;
pub(crate) mod operations;
pub(crate) mod reqwest_client;

use self::backend::BackendState;
pub(crate) use self::backend::{BackEndConnection, BackendEvent, ToBackend};
pub(crate) use reqwest_client::{image_filename, sized_image, ImageKind, ImageSize};

use self::nostr_events::NostrState;

pub enum State {
    End,
    Start {
        keys: Keys,
        backend: BackendState,
    },
    Connected {
        ntp_receiver: tokio::sync::mpsc::Receiver<Result<u64, ntp::Error>>,
        keys: Keys,
        receiver: mpsc::UnboundedReceiver<ToBackend>,
        backend: BackendState,
        nostr: NostrState,
    },
    OnlyNostr {
        keys: Keys,
        receiver: mpsc::UnboundedReceiver<ToBackend>,
        nostr: NostrState,
    },
}
impl State {
    pub fn start(keys: Keys) -> Self {
        let backend = BackendState::new();
        Self::Start { keys, backend }
    }
}

pub fn backend_connect(keys: &Keys) -> Vec<Subscription<BackendEvent>> {
    struct Backend;
    let id = std::any::TypeId::of::<Backend>();

    let database_sub = subscription::unfold(
        id,
        State::start(keys.to_owned()),
        |state| async move {
            match state {
                State::End => iced::futures::future::pending().await,
                State::OnlyNostr {
                    receiver: _,
                    nostr,
                    keys: _,
                } => {
                    if let Err(e) = nostr.logout().await {
                        tracing::error!("{}", e);
                    }
                    (BackendEvent::LoggedOut, State::End)
                }
                State::Start { keys, backend } => {
                    let (sender, receiver) = mpsc::unbounded();
                    let (ntp_sender, ntp_receiver) = tokio::sync::mpsc::channel(1);
                    let nostr = NostrState::new(&keys).await;
                    let req_client = reqwest::Client::new();
                    let db_client = match Database::new(&keys.public_key().to_string()).await {
                        Ok(database) => database,
                        Err(e) => {
                            tracing::error!("Failed to init database");
                            tracing::error!("{}", e);
                            tracing::warn!("Trying again in 2 secs");
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            return (BackendEvent::Disconnected, State::Start { keys, backend });
                        }
                    };

                    let backend = backend.with_db(db_client).with_req(req_client);

                    tracing::info!("Fetching NTP time to sync with system time");
                    spawn_ntp_request(ntp_sender);

                    (
                        BackendEvent::Connected(BackEndConnection::new(sender)),
                        State::Connected {
                            ntp_receiver,
                            receiver,
                            backend,
                            nostr,
                            keys,
                        },
                    )
                }
                State::Connected {
                    mut ntp_receiver,
                    mut receiver,
                    mut backend,
                    mut nostr,
                    keys,
                } => {
                    let event_opt = tokio::select! {
                        ntp_result = ntp_receiver.recv() => {
                            match ntp_result {
                                Some(Ok(ntp_time)) => {
                                    let input = BackEndInput::NtpTime(ntp_time);
                                    match backend_processing(&keys, &mut nostr, &mut backend, input).await {
                                        Ok(event_opt) => event_opt,
                                        Err(e) => Some(BackendEvent::Error(e.to_string()))
                                    }
                                }, Some(Err(e)) => {
                                    tracing::info!("NTP request failed: {}", e);
                                    None
                                }, None => None
                            }
                        }
                        message = receiver.select_next_some() => {
                            if let ToBackend::Logout = message {
                                backend.logout().await;
                                return (BackendEvent::BackendClosed, State::OnlyNostr { receiver, nostr, keys });
                            } else {
                                match process_message(&keys, &mut nostr, &mut backend, message).await {
                                    Ok(event_opt) => event_opt,
                                    Err(e) => Some(BackendEvent::Error(e.to_string()))
                                }
                            }
                        }
                        backend_input = backend.receiver.select_next_some() => {
                            match backend_processing(&keys, &mut nostr, &mut backend, backend_input).await {
                                Ok(event_opt) => event_opt,
                                Err(e) => Some(BackendEvent::Error(e.to_string()))
                            }
                        }
                        notification = nostr.notifications.recv() => {
                            if let Ok(notification) = notification{
                                backend.process_notification(notification).await
                            } else {
                                tracing::info!("Nostr notification failed");
                                None
                            }
                        },
                    };
                    let event = match event_opt {
                        Some(event) => event,
                        None => BackendEvent::Empty,
                    };
                    (
                        event,
                        State::Connected {
                            ntp_receiver,
                            receiver,
                            backend,
                            nostr,
                            keys,
                        },
                    )
                }
            }
        },
    );

    vec![database_sub]
}
