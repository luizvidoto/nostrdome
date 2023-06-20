use std::collections::HashMap;

use iced::widget::{
    button, column, container,
    image::{Handle, Image},
    row, scrollable, text, text_input, Space,
};
use iced::{alignment, Color, Length};
use nostr::{secp256k1::XOnlyPublicKey, EventId};
use once_cell::sync::Lazy;

use crate::{
    components::{
        chat_view::{self, ChatView},
        common_scrollable, inform_card,
    },
    consts::default_profile_image,
    db::{ChannelCache, ProfileCache},
    error::BackendClosed,
    net::{BackEndConnection, BackendEvent, ImageSize, ToBackend},
    style::{self, Theme},
    types::ChatMessage,
    utils::hide_string,
    widget::Element,
};

use super::{route::Route, RouterCommand};

static CHAT_SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static CHAT_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Debug, Clone)]
pub enum Message {
    MemberPressed(XOnlyPublicKey),
    ChatView(chat_view::Message),
    BackPressed,
    EnterChannelPressed,
}
pub struct Member {
    pub pubkey: XOnlyPublicKey,
    pub profile: Option<ProfileCache>,
}
impl Member {
    pub fn name(&self) -> String {
        let default = hide_string(&self.pubkey.to_string(), 4);
        if let Some(profile) = &self.profile {
            return profile
                .metadata
                .display_name
                .clone()
                .unwrap_or(profile.metadata.name.clone().unwrap_or(default));
        }
        default
    }

    fn new(public_key: &XOnlyPublicKey) -> Self {
        Self {
            pubkey: public_key.to_owned(),
            profile: None,
        }
    }

    fn with_profile(profile: ProfileCache) -> Self {
        Self {
            pubkey: profile.public_key.to_owned(),
            profile: Some(profile),
        }
    }
}
pub enum State {
    Loading,
    Loaded {
        cache: ChannelCache,
        chat_list_container: ChatView,
        messages: Vec<ChatMessage>,
        members: HashMap<XOnlyPublicKey, Member>,
    },
}
pub struct Channel {
    is_subscribed: bool,
    channel_id: EventId,
    state: State,
}
impl Channel {
    pub fn matches_id(&self, channel_id: &EventId) -> bool {
        &self.channel_id == channel_id
    }
    pub fn load(
        channel_id: EventId,
        is_subscribed: bool,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        conn.send(ToBackend::FetchChannelCache(channel_id))?;
        conn.send(ToBackend::FetchChannelMessages(channel_id))?;

        Ok(Self {
            is_subscribed,
            channel_id,
            state: State::Loading,
        })
    }
    pub fn loaded(
        cache: ChannelCache,
        is_subscribed: bool,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        conn.send(ToBackend::FetchChannelMessages(cache.channel_id))?;

        let members = cache
            .members
            .iter()
            .map(|public_key| (public_key.to_owned(), Member::new(public_key)))
            .collect();

        Ok(Self {
            channel_id: cache.channel_id,
            is_subscribed,
            state: State::Loaded {
                cache,
                chat_list_container: ChatView::new(),
                messages: vec![],
                members,
            },
        })
    }
    fn update_cache(&mut self, new_cache: ChannelCache) {
        match &mut self.state {
            State::Loading { .. } => (),
            State::Loaded { cache, members, .. } => {
                *members = new_cache
                    .members
                    .iter()
                    .map(|public_key| (public_key.to_owned(), Member::new(public_key)))
                    .collect();
                *cache = new_cache;
            }
        }
    }
    fn name(&self) -> String {
        match &self.state {
            State::Loading { .. } => "Loading...".into(),
            State::Loaded { cache, .. } => cache
                .metadata
                .name
                .clone()
                .unwrap_or(cache.channel_id.to_string()),
        }
    }
}
impl Route for Channel {
    type Message = Message;

    fn backend_event(
        &mut self,
        event: crate::net::BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<super::RouterCommand<Self::Message>, BackendClosed> {
        let command = RouterCommand::new();

        match event {
            BackendEvent::GotChannelCache(cache) => {
                *self = Self::loaded(cache, self.is_subscribed, conn)?;
            }
            BackendEvent::ChannelCacheUpdated(cache) => self.update_cache(cache),
            BackendEvent::ChannelSubscribed(channel_id) => {
                if self.matches_id(&channel_id) {
                    self.is_subscribed = true;
                }
            }
            BackendEvent::ChannelUnsubscribed(channel_id) => {
                if self.matches_id(&channel_id) {
                    self.is_subscribed = false;
                }
            }
            BackendEvent::GotChannelMessages(channel_id, new_messages) => {
                // messages.iter_mut().for_each(|m| {
                //     if let Some(member) = self.members.get(&m.author) {
                //         m.display_name = member.name();
                //     }
                // });
                if self.matches_id(&channel_id) {
                    match &mut self.state {
                        State::Loading => (),
                        State::Loaded { messages, .. } => {
                            *messages = new_messages;
                        }
                    }
                }
            }
            BackendEvent::ReceivedChannelMessage(channel_id, new_message) => {
                // match &mut message {
                //     ChatMessage::UserMessage(_) => (),
                //     ChatMessage::ContactMessage { author, .. } => {
                //         if let Some(member) = self.members.get(author) {
                //             message.display_name = member.name();
                //         }
                //     }
                // }
                if self.matches_id(&channel_id) {
                    match &mut self.state {
                        State::Loading => (),
                        State::Loaded { messages, .. } => {
                            messages.push(new_message);
                            messages.sort_by(|a, b| a.display_time().cmp(&b.display_time()))
                        }
                    }
                }
            }

            BackendEvent::UpdatedMetadata(pubkey) => match &mut self.state {
                State::Loading => (),
                State::Loaded { members, .. } => {
                    if members.contains_key(&pubkey) {
                        conn.send(ToBackend::FetchProfileCache(pubkey))?;
                    }
                }
            },
            BackendEvent::GotProfileCache(pubkey, profile) => match &mut self.state {
                State::Loading => (),
                State::Loaded {
                    members, messages, ..
                } => {
                    if let Some(member) = members.get_mut(&pubkey) {
                        *member = Member::with_profile(profile);

                        messages.iter_mut().for_each(|m| {
                            m.update_display_name(&member.pubkey, member.name());
                        });
                    }
                }
            },
            _ => {}
        }

        Ok(command)
    }

    fn update(
        &mut self,
        message: Self::Message,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let mut command = RouterCommand::new();

        match message {
            Message::MemberPressed(member) => {
                // show modal?
                tracing::info!("Member pressed: {:?}", member)
            }
            Message::BackPressed => {
                // Todo: make go back work
                command.change_route(super::GoToView::Chat);
            }
            Message::EnterChannelPressed => {
                conn.send(ToBackend::SubscribeToChannel(self.channel_id.to_owned()))?;
            }
            Message::ChatView(ch_msg) => match ch_msg {
                chat_view::Message::DMSentPress(_) => tracing::info!("DM sent!"),
                chat_view::Message::DMNMessageChange(_) => {
                    tracing::info!("DMNMessageChange")
                }
                chat_view::Message::GotChatSize(_, _) => tracing::info!("GotChatSize"),
                chat_view::Message::Scrolled(_) => tracing::info!("Scrolled"),
                chat_view::Message::OpenContactProfile => {
                    tracing::info!("OpenContactProfile")
                }
                chat_view::Message::ChatRightClick(_, _) => {
                    tracing::info!("ChatRightClick")
                }
                chat_view::Message::ChannelOpenModalPressed => {
                    tracing::info!("ChannelOpenModalPressed")
                }
                chat_view::Message::ChannelSearchPressed => {
                    tracing::info!("ChannelSearchPressed")
                }
                chat_view::Message::ChannelMenuPressed => {
                    tracing::info!("ChannelMenuPressed")
                }
                chat_view::Message::ChannelUserNamePressed(author) => {
                    tracing::info!("ChannelUserNamePressed: {}", author)
                }
            },
        }

        Ok(command)
    }

    fn view(&self, _selected_theme: Option<Theme>) -> Element<'_, Self::Message> {
        match &self.state {
            State::Loading { .. } => inform_card("Loading Channel", "Please wait"),
            State::Loaded {
                cache,
                chat_list_container,
                messages,
                members,
            } => {
                // let members_list = make_member_list(self.channel.members.iter(), Message::MemberPressed);

                let members_list = members
                    .iter()
                    .fold(column![].spacing(5), |col, (_, member)| {
                        col.push(member_btn(member))
                    });
                let members_list = container(common_scrollable(
                    column![text("Members").size(24), members_list].spacing(10),
                ))
                .padding(10)
                .height(Length::Fill)
                .width(MEMBERS_LIST_WIDTH)
                .style(style::Container::Foreground);

                let chat_view = chat_list_container
                    .channel_view(
                        &CHAT_SCROLLABLE_ID,
                        &CHAT_INPUT_ID,
                        messages,
                        &self.name(),
                        members.len() as i32,
                    )
                    .map(Message::ChatView);

                let content = row![members_list, chat_view];

                let show_join: Element<_> = if self.is_subscribed {
                    text("").into()
                } else {
                    let back_btn = button("Back")
                        .on_press(Message::BackPressed)
                        .style(style::Button::HighlightButton);
                    let subscribe_btn = button(text(&format!("Enter {}", self.name())))
                        .on_press(Message::EnterChannelPressed)
                        .style(style::Button::HighlightButton);
                    container(
                        row![
                            back_btn,
                            Space::with_width(Length::Fill),
                            text("Subscribe to this channel")
                                .style(style::Text::Color(Color::WHITE)),
                            Space::with_width(10),
                            subscribe_btn,
                            Space::with_width(Length::Fill)
                        ]
                        .align_items(alignment::Alignment::Center),
                    )
                    .style(style::Container::Highlight)
                    .padding(10)
                    .into()
                };

                column![show_join, content].into()
            }
        }
    }
}

fn member_btn(member: &Member) -> Element<'_, Message> {
    let content = row![
        container(Image::new(Handle::from_memory(default_profile_image(
            ImageSize::Small
        ))))
        .width(30)
        .height(30),
        text(member.name()).size(15)
    ]
    .spacing(5);

    button(content)
        .on_press(Message::MemberPressed(member.pubkey.to_owned()))
        .style(style::Button::ContactCard)
        .width(Length::Fill)
        .into()
}

const MEMBERS_LIST_WIDTH: u16 = 200;
