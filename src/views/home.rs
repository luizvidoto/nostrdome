use iced::widget::{button, column, container, image, image::Handle, row, Rule};
use iced::{alignment, Length, Subscription};
use nostr::EventId;
use status_bar::StatusBar;

use crate::components::{invisible_scrollable, status_bar};
use crate::consts::default_channel_image;
use crate::db::{ChannelCache, DbContact};
use crate::error::BackendClosed;
use crate::icon::{settings_icon, wand_icon};
use crate::net::{BackEndConnection, BackendEvent, ImageSize, ToBackend};

use crate::types::ChannelResult;
use crate::widget::Text;
use crate::{
    icon::{home_icon, search_icon},
    style,
    widget::Element,
};

use super::route::Route;
use super::{channel, chat, color_palettes, find_channels, GoToView, RouterCommand};

pub enum HomeGoTo {
    Channel(ChannelResult),
}

#[derive(Debug, Clone)]
pub enum Message {
    DMsPressed,
    FindChannelsPressed,
    SettingsPressed,
    ColorPalettePressed,
    MenuChannelBtnPressed(EventId),
    Dms(chat::Message),
    FindChannels(find_channels::Message),
    StatusBar(status_bar::Message),
    ColorPalette(color_palettes::Message),
    Channel(channel::Message),
}
pub struct State {
    active_view: ViewState,
    channels_subscribed: Vec<ChannelMenuBtn>,
    status_bar: StatusBar,
}

impl State {
    pub(crate) fn chat(conn: &mut BackEndConnection) -> Result<Self, BackendClosed> {
        conn.send(ToBackend::FetchSubscribedChannels)?;
        Ok(Self {
            status_bar: StatusBar::new(),
            active_view: ViewState::DMs {
                state: chat::State::new(conn)?,
            },
            channels_subscribed: Vec::new(),
        })
    }
    pub(crate) fn chat_to(
        db_contact: DbContact,
        conn: &mut BackEndConnection,
    ) -> Result<Self, BackendClosed> {
        conn.send(ToBackend::FetchSubscribedChannels)?;
        Ok(Self {
            status_bar: StatusBar::new(),
            active_view: ViewState::DMs {
                state: chat::State::chat_to(db_contact, conn)?,
            },
            channels_subscribed: Vec::new(),
        })
    }
    pub(crate) fn find_channels(conn: &mut BackEndConnection) -> Result<State, BackendClosed> {
        conn.send(ToBackend::FetchSubscribedChannels)?;
        Ok(Self {
            status_bar: StatusBar::new(),
            active_view: ViewState::FindChannel {
                state: find_channels::State::new(conn),
            },
            channels_subscribed: Vec::new(),
        })
    }
}

impl Route for State {
    type Message = Message;
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            self.active_view.subscription(),
            self.status_bar.subscription().map(Message::StatusBar),
        ])
    }
    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        match event.clone() {
            BackendEvent::GotSubscribedChannels(channels) => {
                self.channels_subscribed = channels
                    .into_iter()
                    .map(ChannelMenuBtn::with_cache)
                    .collect()
            }
            BackendEvent::ChannelSubscribed(channel_id) => {
                self.channels_subscribed
                    .push(ChannelMenuBtn::new(channel_id));
                conn.send(ToBackend::FetchChannelCache(channel_id))?;
            }
            BackendEvent::ChannelUnsubscribed(channel_id) => {
                self.channels_subscribed
                    .retain(|btn| btn.channel_id != channel_id);
            }
            BackendEvent::GotChannelCache(cache) | BackendEvent::ChannelCacheUpdated(cache) => {
                if let Some(btn) = self
                    .channels_subscribed
                    .iter_mut()
                    .find(|btn| btn.channel_id == cache.channel_id)
                {
                    btn.update_cache(cache);
                }
            }
            _ => (),
        }

        let mut commands = self.active_view.backend_event(event.clone(), conn)?;

        let cmd = self.status_bar.backend_event(event, conn);
        commands.push(cmd.map(Message::StatusBar));

        Ok(commands)
    }
    fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let mut commands = RouterCommand::new();

        match message {
            Message::ColorPalettePressed => match self.active_view {
                ViewState::ColorPalettes { .. } => (),
                _ => {
                    self.active_view = ViewState::ColorPalettes {
                        state: color_palettes::State::new(),
                    }
                }
            },
            Message::DMsPressed => match self.active_view {
                ViewState::DMs { .. } => (),
                _ => {
                    self.active_view = ViewState::DMs {
                        state: chat::State::new(conn)?,
                    }
                }
            },
            Message::FindChannelsPressed => match self.active_view {
                ViewState::FindChannel { .. } => (),
                _ => {
                    self.active_view = ViewState::FindChannel {
                        state: find_channels::State::new(conn),
                    }
                }
            },
            Message::MenuChannelBtnPressed(channel_id) => match &mut self.active_view {
                ViewState::Channel { state } if state.matches_id(&channel_id) => (),
                _ => {
                    self.active_view = ViewState::Channel {
                        state: channel::Channel::load(channel_id, true, conn)?,
                    }
                }
            },
            Message::StatusBar(status_msg) => {
                return Ok(self
                    .status_bar
                    .update(status_msg, conn)?
                    .map(Message::StatusBar));
            }
            Message::FindChannels(msg) => {
                if let ViewState::FindChannel { state } = &mut self.active_view {
                    if let Some(router_message) = state.update(msg, conn)? {
                        match router_message {
                            HomeGoTo::Channel(result) => {
                                let is_subscribed = self
                                    .channels_subscribed
                                    .iter()
                                    .any(|btn| btn.channel_id == result.cache.channel_id);

                                self.active_view = ViewState::Channel {
                                    state: channel::Channel::loaded(
                                        result.cache,
                                        is_subscribed,
                                        conn,
                                    )?,
                                }
                            }
                        }
                    }
                }
            }
            Message::SettingsPressed => commands.change_route(GoToView::Settings),
            Message::ColorPalette(msg) => {
                if let ViewState::ColorPalettes { state } = &mut self.active_view {
                    return Ok(state.update(msg, conn)?.map(Message::ColorPalette));
                }
            }
            Message::Channel(msg) => {
                if let ViewState::Channel { state } = &mut self.active_view {
                    return Ok(state.update(msg, conn)?.map(Message::Channel));
                }
            }
            Message::Dms(msg) => {
                if let ViewState::DMs { state } = &mut self.active_view {
                    return Ok(state.update(msg, conn)?.map(Message::Dms));
                }
            }
        }

        Ok(commands)
    }
    fn view(&self, selected_theme: Option<style::Theme>) -> Element<Self::Message> {
        let color_palette_btn = make_menu_btn(
            self.active_view.is_color_palette_view(),
            wand_icon,
            Message::ColorPalettePressed,
        );
        let dm_btn = make_menu_btn(self.active_view.is_dms(), home_icon, Message::DMsPressed);
        let find_ch_btn = make_menu_btn(
            self.active_view.is_find_channel(),
            search_icon,
            Message::FindChannelsPressed,
        );
        let settings_btn = make_menu_btn(false, settings_icon, Message::SettingsPressed);
        let spacer = container(Rule::horizontal(2))
            .padding([0, 10])
            .width(Length::Fill);

        let channel_buttons =
            self.channels_subscribed
                .iter()
                .fold(column![].spacing(2), |col, btn| {
                    let is_active = self.active_view.is_channel_selected(&btn.channel_id);
                    let message = Message::MenuChannelBtnPressed(btn.channel_id.to_owned());
                    col.push(make_channel_menu_btn(
                        is_active,
                        btn.image_handle.to_owned(),
                        message,
                    ))
                });

        let nav_bar = container(
            column![
                container(invisible_scrollable(
                    column![
                        dm_btn,
                        spacer,
                        find_ch_btn,
                        color_palette_btn,
                        channel_buttons
                    ]
                    .spacing(2)
                ),)
                .height(Length::Fill),
                settings_btn,
            ]
            .spacing(2),
        )
        .padding([2, 0])
        .style(style::Container::Background)
        .width(NAVBAR_WIDTH)
        .height(Length::Fill);

        let status_bar = self.status_bar.view().map(Message::StatusBar);

        let active_view = column![
            container(self.active_view.view(selected_theme))
                .width(Length::Fill)
                .height(Length::Fill),
            status_bar
        ];

        row![nav_bar, active_view].into()
    }
}

fn make_menu_btn<'a, M: 'a + Clone>(
    is_active: bool,
    icon: impl Fn() -> Text<'a>,
    message: M,
) -> Element<'a, M> {
    let style = if is_active {
        style::Button::ActiveMenuBtn
    } else {
        style::Button::MenuBtn
    };

    container(
        button(
            icon()
                .size(ICON_SIZE)
                .width(Length::Fill)
                .height(Length::Fill)
                .vertical_alignment(alignment::Vertical::Center)
                .horizontal_alignment(alignment::Horizontal::Center),
        )
        .style(style)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(message),
    )
    .padding([PADDING_V, PADDING_H])
    .width(NAVBAR_WIDTH)
    .height(NAVBAR_WIDTH)
    .into()
}

fn make_channel_menu_btn<'a, M: 'a + Clone>(
    is_active: bool,
    image_handle: Handle,
    message: M,
) -> Element<'a, M> {
    let style = if is_active {
        style::Button::ActiveMenuBtn
    } else {
        style::Button::MenuBtn
    };

    container(
        button(image(image_handle).width(Length::Fill).height(Length::Fill))
            .style(style)
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(message),
    )
    .padding([PADDING_V, PADDING_H])
    .width(NAVBAR_WIDTH)
    .height(NAVBAR_WIDTH)
    .into()
}

pub enum ViewState {
    Channel { state: channel::Channel },
    ColorPalettes { state: color_palettes::State },
    DMs { state: chat::State },
    FindChannel { state: find_channels::State },
}
impl ViewState {
    pub fn is_dms(&self) -> bool {
        matches!(self, ViewState::DMs { .. })
    }
    pub fn is_find_channel(&self) -> bool {
        matches!(self, ViewState::FindChannel { .. })
    }
    fn is_color_palette_view(&self) -> bool {
        matches!(self, ViewState::ColorPalettes { .. })
    }
    fn is_channel_selected(&self, channel_id: &EventId) -> bool {
        match self {
            ViewState::Channel { state } => state.matches_id(channel_id),
            _ => false,
        }
    }
}
impl Route for ViewState {
    type Message = Message;

    fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Self::Message>, BackendClosed> {
        let command = match self {
            ViewState::Channel { state } => state.backend_event(event, conn)?.map(Message::Channel),
            ViewState::ColorPalettes { state } => {
                state.backend_event(event, conn)?.map(Message::ColorPalette)
            }
            ViewState::DMs { state } => state.backend_event(event, conn)?.map(Message::Dms),
            ViewState::FindChannel { state } => {
                state.backend_event(event, conn)?.map(Message::FindChannels)
            }
        };

        Ok(command)
    }
    fn subscription(&self) -> Subscription<Self::Message> {
        match self {
            ViewState::ColorPalettes { state } => state.subscription().map(Message::ColorPalette),
            ViewState::Channel { state } => state.subscription().map(Message::Channel),
            ViewState::DMs { state } => state.subscription().map(Message::Dms),
            ViewState::FindChannel { state: _ } => Subscription::none(),
        }
    }
    fn view(&self, selected_theme: Option<style::Theme>) -> Element<Self::Message> {
        match self {
            ViewState::ColorPalettes { state } => {
                state.view(selected_theme).map(Message::ColorPalette)
            }
            ViewState::Channel { state } => state.view(selected_theme).map(Message::Channel),
            ViewState::DMs { state } => state.view(selected_theme).map(Message::Dms),
            ViewState::FindChannel { state } => {
                state.view(selected_theme).map(Message::FindChannels)
            }
        }
    }
}

pub struct ChannelMenuBtn {
    channel_id: EventId,
    cache: Option<ChannelCache>,
    image_handle: Handle,
}
impl ChannelMenuBtn {
    pub fn new(channel_id: EventId) -> Self {
        Self {
            channel_id,
            cache: None,
            image_handle: Handle::from_memory(default_channel_image(IMAGE_SIZE)),
        }
    }
    pub fn with_cache(cache: ChannelCache) -> Self {
        let image_handle = cache
            .image_cache
            .as_ref()
            .map(|image| {
                let path = image.sized_image(IMAGE_SIZE);
                Handle::from_path(path)
            })
            .unwrap_or(Handle::from_memory(default_channel_image(IMAGE_SIZE)));
        Self {
            channel_id: cache.channel_id,
            cache: Some(cache),
            image_handle,
        }
    }

    fn update_cache(&mut self, cache: ChannelCache) {
        let image_handle = cache
            .image_cache
            .as_ref()
            .map(|image| {
                let path = image.sized_image(IMAGE_SIZE);
                Handle::from_path(path)
            })
            .unwrap_or(Handle::from_memory(default_channel_image(IMAGE_SIZE)));
        self.image_handle = image_handle;
        self.cache = Some(cache);
    }
}

const NAVBAR_WIDTH: u16 = 55;
const PADDING_V: u16 = 6;
const PADDING_H: u16 = 6;
const ICON_SIZE: u16 = 26;

const IMAGE_SIZE: ImageSize = ImageSize::Small;
