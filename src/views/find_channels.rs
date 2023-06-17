use std::collections::HashMap;

use iced::widget::{button, column, container, image, row, text, Space};
use iced::Length;
use iced_native::widget::text_input;
use nostr::EventId;

use crate::components::common_scrollable;
use crate::components::text::title;
use crate::consts::{MEDIUM_CHANNEL_IMG_HEIGHT, MEDIUM_CHANNEL_IMG_WIDTH, YMD_FORMAT};
use crate::error::BackendClosed;
use crate::net::{BackEndConnection, BackendEvent, ToBackend};
use crate::types::ChannelResult;
use crate::views::RouterCommand;
use crate::widget::Rule;
use crate::{icon::search_icon, style, widget::Element};

use super::home::HomeRouterMessage;

#[derive(Debug, Clone)]
pub enum Message {
    SearchInputChanged(String),
    SubmitPress,
    ChannelPressed(ChannelResult),
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
    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
    ) -> Result<Option<HomeRouterMessage>, BackendClosed> {
        match message {
            Message::ChannelPressed(result) => {
                return Ok(Some(HomeRouterMessage::GoToChannel(result)));
            }
            Message::SearchInputChanged(text) => {
                self.search_input_value = text;
            }
            Message::SubmitPress => {
                self.searching = false;
                self.search_results = HashMap::new();
                conn.send(ToBackend::FindChannels(self.search_input_value.clone()))?;
            }
        }

        Ok(None)
    }
    pub fn backend_event(
        &mut self,
        event: BackendEvent,
        conn: &mut BackEndConnection,
    ) -> Result<RouterCommand<Message>, BackendClosed> {
        let commands = RouterCommand::new();

        match event {
            BackendEvent::SearchingForChannels => {
                self.searching = true;
            }
            BackendEvent::SearchChannelResult(result) => {
                self.search_results
                    .insert(result.channel_id.to_owned(), result);
            }
            BackendEvent::EOSESearchChannels(_url) => {
                self.searching = false;
            }
            BackendEvent::SearchChannelCacheUpdated(channel_id, cache) => {
                if let Some(result) = self.search_results.get_mut(&channel_id) {
                    result.update_cache(cache, conn)?;
                }
            }
            BackendEvent::SearchChannelMessage(channel_id, pubkey) => {
                self.search_results
                    .get_mut(&channel_id)
                    .map(|r| r.message(pubkey));
            }
            BackendEvent::LoadingChannelDetails(_url, channel_id) => {
                self.search_results
                    .get_mut(&channel_id)
                    .map(|r| r.loading_details());
            }
            BackendEvent::EOSESearchChannelsDetails(_url, prefixed_id) => {
                let channel_id_str = prefixed_id.to_string();
                for (id, result) in self.search_results.iter_mut() {
                    if id.to_string().starts_with(&channel_id_str) {
                        result.done_loading();
                        break;
                    }
                }
            }
            BackendEvent::ImageDownloaded(image) => {
                if let Some((_, result)) = self
                    .search_results
                    .iter_mut()
                    .find(|(_, r)| r.cache.last_event_hash() == &image.event_hash)
                {
                    result.update_image(&image);
                }
            }
            _ => (),
        }

        Ok(commands)
    }
    pub fn view(&self, _selected_theme: Option<style::Theme>) -> Element<Message> {
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
            .fold(column![], |acc, (_, result)| {
                acc.push(channel_card(
                    result,
                    Message::ChannelPressed(result.to_owned()),
                ))
            });

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

fn channel_card<'a, M: 'a + Clone>(channel: &ChannelResult, on_channel_press: M) -> Element<'a, M> {
    let image_container = container(image(channel.image_handle.to_owned()))
        .width(MEDIUM_CHANNEL_IMG_WIDTH)
        .height(MEDIUM_CHANNEL_IMG_HEIGHT)
        .center_x()
        .center_y();

    let name_about_ct = container(common_scrollable(
        column![
            text(channel.name()).size(22),
            text(channel.about()).size(18),
            text(&channel.relay_url.to_string()).size(14),
        ]
        .spacing(5),
    ))
    .max_height(MEDIUM_CHANNEL_IMG_HEIGHT - BOTTOM_ROW_HEIGHT - 10);

    let bottom_row_ct = container(
        row![
            text(format!("Members: {}", &channel.members.len())).size(14),
            text(format!(
                "Created: {}",
                channel.created_at().format(YMD_FORMAT)
            ))
            .size(14),
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
            button(row![image_container, channel_about, loading_ct].spacing(20))
                .padding(0)
                .style(style::Button::Invisible)
                .on_press(on_channel_press),
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
