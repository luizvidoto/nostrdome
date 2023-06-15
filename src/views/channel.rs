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
        chat_list_container::{self, ChatListContainer},
        common_scrollable,
    },
    consts::default_profile_image,
    db::ProfileCache,
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
    ChatListMessage(chat_list_container::Message),
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

    fn with_profile(profile: ProfileCache) -> Self {
        Self {
            pubkey: profile.public_key.to_owned(),
            profile: Some(profile),
        }
    }
}
pub struct State {
    // pub channel: ChannelResult,
    pub channel_id: EventId,
    name: String,
    is_subscribed: bool,
    chat_list_container: ChatListContainer,
    messages: Vec<ChatMessage>,
    members: HashMap<XOnlyPublicKey, Member>,
}
impl State {
    pub fn new(
        channel_id: EventId,
        is_subscribed: bool,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        conn.send(ToBackend::FetchChannelMembers(channel_id.clone()))?;
        conn.send(ToBackend::FetchChannelMessages(channel_id.clone()))?;
        conn.send(ToBackend::FetchChannelCache(channel_id.clone()))?;

        Ok(Self {
            // channel,
            channel_id,
            name: "".into(),
            is_subscribed,
            chat_list_container: ChatListContainer::new(),
            messages: vec![],
            members: HashMap::new(),
        })
    }
}
impl Route for State {
    type Message = Message;

    fn backend_event(
        &mut self,
        event: crate::net::BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<super::RouterCommand<Self::Message>, BackendClosed> {
        let command = RouterCommand::new();

        match event {
            BackendEvent::GotChannelCache(cache) | BackendEvent::ChannelCacheUpdated(cache) => {
                self.name = cache.metadata.name.unwrap_or("".into());
            }
            BackendEvent::ChannelSubscribed(_channel_id) => {
                self.is_subscribed = true;
            }
            BackendEvent::ChannelUnsubscribed(_channel_id) => {
                self.is_subscribed = false;
            }
            BackendEvent::GotChannelMessages(_channel_id, mut messages) => {
                messages.iter_mut().for_each(|m| {
                    if let Some(member) = self.members.get(&m.author) {
                        m.display_name = member.name();
                    }
                });
                self.messages = messages;
            }
            BackendEvent::ReceivedChannelMessage(mut message) => {
                if let Some(member) = self.members.get(&message.author) {
                    message.display_name = member.name();
                }
                self.messages.push(message);
                self.messages
                    .sort_by(|a, b| a.display_time.cmp(&b.display_time))
            }
            BackendEvent::GotChannelMembers(members) => {
                self.members = members
                    .into_iter()
                    .map(|profile| (profile.public_key.to_owned(), Member::with_profile(profile)))
                    .collect();
            }
            BackendEvent::GotProfileCache(pubkey, profile) => {
                if let Some(member) = self.members.get_mut(&pubkey) {
                    member.profile = Some(profile);
                    self.messages.iter_mut().for_each(|m| {
                        if m.author == pubkey {
                            m.display_name = member.name();
                        }
                    });
                }
            }
            BackendEvent::UpdatedMetadata(pubkey) => {
                if self.members.contains_key(&pubkey) {
                    conn.send(ToBackend::FetchProfileCache(pubkey))?;
                }
            }
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
                command.change_route(super::RouterMessage::GoToChat);
            }
            Message::EnterChannelPressed => {
                conn.send(ToBackend::SubscribeToChannel(self.channel_id.to_owned()))?;
            }
            Message::ChatListMessage(ch_msg) => match ch_msg {
                chat_list_container::Message::DMSentPress(_) => tracing::info!("DM sent!"),
                chat_list_container::Message::DMNMessageChange(_) => {
                    tracing::info!("DMNMessageChange")
                }
                chat_list_container::Message::GotChatSize(_, _) => tracing::info!("GotChatSize"),
                chat_list_container::Message::Scrolled(_) => tracing::info!("Scrolled"),
                chat_list_container::Message::OpenContactProfile => {
                    tracing::info!("OpenContactProfile")
                }
                chat_list_container::Message::ChatRightClick(_, _) => {
                    tracing::info!("ChatRightClick")
                }
                chat_list_container::Message::ChannelOpenModalPressed => {
                    tracing::info!("ChannelOpenModalPressed")
                }
                chat_list_container::Message::ChannelSearchPressed => {
                    tracing::info!("ChannelSearchPressed")
                }
                chat_list_container::Message::ChannelMenuPressed => {
                    tracing::info!("ChannelMenuPressed")
                }
                chat_list_container::Message::ChannelUserNamePressed(author) => {
                    tracing::info!("ChannelUserNamePressed: {}", author)
                }
            },
        }

        Ok(command)
    }

    fn view(&self, _selected_theme: Option<Theme>) -> Element<'_, Self::Message> {
        // let members_list = make_member_list(self.channel.members.iter(), Message::MemberPressed);

        let members_list = self
            .members
            .iter()
            .fold(column![].spacing(5), |col, (_, member)| {
                col.push(member_btn(member))
            });
        let members_list = common_scrollable(
            container(column![text("Members").size(24), members_list].spacing(10))
                .padding(10)
                .width(MEMBERS_LIST_WIDTH)
                .height(Length::Fill)
                .style(style::Container::Foreground),
        );

        let chat_view = self
            .chat_list_container
            .channel_view(
                &CHAT_SCROLLABLE_ID,
                &CHAT_INPUT_ID,
                &self.messages,
                &self.name,
                self.members.len() as i32,
            )
            .map(Message::ChatListMessage);

        let content = row![members_list, chat_view];

        let show_join: Element<_> = if self.is_subscribed {
            text("").into()
        } else {
            let back_btn = button("Back")
                .on_press(Message::BackPressed)
                .style(style::Button::HighlightButton);
            let subscribe_btn = button(text(&format!("Enter {}", self.name)))
                .on_press(Message::EnterChannelPressed)
                .style(style::Button::HighlightButton);
            container(
                row![
                    back_btn,
                    Space::with_width(Length::Fill),
                    text("Subscribe to this channel").style(style::Text::Color(Color::WHITE)),
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

// fn make_member_list<'a, M: 'a + Clone, I, F>(list: I, member_press: F) -> Element<'a, M>
// where
//     I: IntoIterator<Item = &'a XOnlyPublicKey>,
//     F: Fn(XOnlyPublicKey) -> M + 'a,
// {

// }
