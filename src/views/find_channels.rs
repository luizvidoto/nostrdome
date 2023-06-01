use std::collections::HashMap;

use iced::widget::{button, column, container, image, row, text, tooltip, Space};
use iced::{clipboard, Alignment, Command, Length, Subscription};
use iced_native::widget::text_input;
use nostr::EventId;
use url::Url;

use crate::components::common_scrollable;
use crate::components::text::title;
use crate::consts::{MEDIUM_CHANNEL_IMG_HEIGHT, MEDIUM_CHANNEL_IMG_WIDTH, YMD_FORMAT};
use crate::icon::copy_icon;
use crate::net::{BackEndConnection, BackendEvent, ImageKind, ToBackend};
use crate::types::ChannelResult;
use crate::views::RouterCommand;
use crate::widget::Rule;
use crate::{icon::search_icon, style, widget::Element};

#[derive(Debug, Clone)]
pub enum Message {
    SearchInputChanged(String),
    SubmitPress,
    CopyChannelId(nostr::EventId),
}
pub struct State {
    search_results: HashMap<EventId, ChannelResult>,
    search_input_value: String,
    searching: bool,
}
impl State {
    pub fn new(_conn: &mut BackEndConnection) -> Self {
        Self {
            search_results: HashMap::new(),
            search_input_value: String::new(),
            searching: false,
        }
    }
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
    pub(crate) fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        let mut commands = RouterCommand::new();
        match message {
            Message::CopyChannelId(id) => {
                commands.push(clipboard::write(id.to_hex()));
            }
            Message::SearchInputChanged(text) => {
                self.search_input_value = text;
            }
            Message::SubmitPress => {
                self.searching = false;
                self.search_results = HashMap::new();
                conn.send(ToBackend::FindChannels(self.search_input_value.clone()))
            }
        }
        commands
    }
    pub(crate) fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> RouterCommand<Message> {
        let commands = RouterCommand::new();

        match event {
            BackendEvent::SearchingForChannels => {
                self.searching = true;
            }
            BackendEvent::SearchChannelResult(result) => {
                self.search_results.insert(result.id, result);
            }
            BackendEvent::EOSESearchChannels(url) => {
                self.searching = false;
            }
            BackendEvent::SearchChannelMetaUpdate(channel_id, ns_event) => {
                if let Some(result) = self.search_results.get_mut(&channel_id) {
                    if let Err(e) = result.update_meta(ns_event) {
                        tracing::error!("Error updating channel meta: {:?}", e);
                    }
                }
            }
            BackendEvent::SearchChannelMessage(channel_id, ns_event) => {
                self.search_results
                    .get_mut(&channel_id)
                    .map(|r| r.message(ns_event));
            }
            BackendEvent::LoadingChannelDetails(_url, channel_id) => {
                self.search_results
                    .get_mut(&channel_id)
                    .map(|r| r.loading_details());
            }
            BackendEvent::EOSESearchChannelsDetails(_url, channel_id) => {
                if let Some(result) = self.search_results.get_mut(&channel_id) {
                    result.done_loading();

                    if let Some(image_url) = &result.picture {
                        match Url::parse(image_url) {
                            Ok(image_url_parsed) => conn.send(ToBackend::DownloadImage {
                                identifier: result.id.to_string(),
                                image_url: image_url_parsed,
                                kind: ImageKind::Channel,
                            }),
                            Err(e) => {
                                tracing::error!("Error parsing image url: {:?}", e);
                            }
                        }
                    }
                }
            }
            BackendEvent::ImageDownloaded(image) => {
                if let Some((_, result)) = self
                    .search_results
                    .iter_mut()
                    .find(|(_, r)| r.picture == Some(image.url.to_string()))
                {
                    result.update_image(image);
                }
            }
            _ => (),
        }

        commands
    }
    pub fn view(&self) -> Element<Message> {
        let title = title("Find Channels");

        let searching_text = if self.searching {
            text("Searching for channels...").size(18)
        } else {
            text("")
        };

        let search_input = container(
            row![
                text_input("Channel id", &self.search_input_value)
                    .width(Length::Fill)
                    .on_input(Message::SearchInputChanged)
                    .on_submit(Message::SubmitPress)
                    .size(26),
                button(search_icon().size(20))
                    .on_press(Message::SubmitPress)
                    .style(style::Button::MenuBtn)
                    .width(30)
            ]
            .width(Length::Fill)
            .spacing(10),
        )
        .max_width(MAX_WIDTH_RESULT);

        let results_container = self
            .search_results
            .iter()
            .fold(column![], |acc, (_, result)| acc.push(make_results(result)));

        common_scrollable(
            container(column![
                title,
                search_input,
                searching_text,
                results_container
            ])
            .width(Length::Fill)
            .padding([20, 20, 0, 20]),
        )
        .into()
    }
}

fn make_results<'a>(channel: &ChannelResult) -> Element<'a, Message> {
    let image_container = container(image(channel.image_handle.to_owned()))
        .width(MEDIUM_CHANNEL_IMG_WIDTH)
        .height(MEDIUM_CHANNEL_IMG_HEIGHT)
        .center_x()
        .center_y();

    let name_about_ct = container(common_scrollable(
        column![
            text(channel.name.as_ref().unwrap_or(&"Nameless".into())).size(22),
            text(channel.about.as_ref().unwrap_or(&"".into())).size(18),
            text(&channel.url.to_string()).size(14),
        ]
        .spacing(5),
    ))
    .max_height(MEDIUM_CHANNEL_IMG_HEIGHT - BOTTOM_ROW_HEIGHT - 10);

    let copy_btn = tooltip(
        button(copy_icon().size(14))
            .padding(2)
            .on_press(Message::CopyChannelId(channel.id.to_owned()))
            .style(style::Button::Primary),
        "Copy Channel Id",
        tooltip::Position::Top,
    )
    .style(style::Container::TooltipBg);

    let bottom_row_ct = container(
        row![
            text(format!("Members: {}", &channel.members.len())).size(14),
            text(format!(
                "Created: {}",
                &channel.created_at.format(YMD_FORMAT)
            ))
            .size(14),
            copy_btn
        ]
        .spacing(4),
    )
    .height(BOTTOM_ROW_HEIGHT);

    let channel_about = container(
        column![
            name_about_ct,
            Space::with_height(Length::Fill),
            bottom_row_ct
        ]
        .spacing(5),
    )
    .center_y()
    .height(MEDIUM_CHANNEL_IMG_HEIGHT)
    .width(Length::Fill);

    let loading_ct = if channel.loading_details {
        container(text("Loading...").size(26))
            .width(100)
            .height(100)
    } else {
        container(text(""))
    };

    container(
        column![
            row![image_container, channel_about, loading_ct].spacing(20),
            Rule::horizontal(10)
        ]
        .spacing(10),
    )
    .padding(10)
    .max_width(MAX_WIDTH_RESULT)
    .into()
}

const BOTTOM_ROW_HEIGHT: u16 = 20;
const MAX_WIDTH_RESULT: u16 = 800;
