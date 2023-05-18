use iced::alignment::Horizontal;
use iced::widget::{button, column, container, image, row, text};
use iced::{Alignment, Length, Subscription};
use iced_aw::{Card, Modal};

use crate::components::text_input_group::TextInputGroup;
use crate::components::{common_scrollable, inform_card, relay_row, RelayRow};
use crate::consts::{RELAYS_IMAGE, RELAY_SUGGESTIONS, WELCOME_IMAGE};
use crate::db::DbRelay;
use crate::icon::{regular_circle_icon, solid_circle_icon};
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::utils::add_ellipsis_trunc;
use crate::{components::text::title, widget::Element};

use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub enum Message {
    RelayMessage(relay_row::MessageWrapper),
    ToNextStep,
    ToPreviousStep,
    ToApp,
    Exit,
    AddRelay(nostr_sdk::Url),
    AddOtherPress,

    // Add Relay Modal
    AddRelayInputChange(String),
    AddRelaySubmit(String),
    AddRelayCancelButtonPressed,
    CloseAddRelayModal,
}

#[derive(Debug, Clone)]
pub enum ModalState {
    AddRelay { relay_url: String, is_invalid: bool },
    Off,
}

impl ModalState {
    pub fn view<'a>(&'a self, underlay: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        match self {
            ModalState::AddRelay {
                relay_url,
                is_invalid,
            } => Modal::new(true, underlay.into(), move || {
                let mut add_relay_input =
                    TextInputGroup::new("Relay Address", &relay_url, Message::AddRelayInputChange)
                        .placeholder("wss://my-relay.com")
                        .on_submit(Message::AddRelaySubmit(relay_url.clone()));

                if *is_invalid {
                    add_relay_input = add_relay_input.invalid("Relay address is invalid");
                }

                let modal_body: Element<_> = container(add_relay_input.build()).into();
                Card::new(text("Add Relay"), modal_body)
                    .foot(
                        row![
                            button(text("Cancel").horizontal_alignment(Horizontal::Center),)
                                .width(Length::Fill)
                                .on_press(Message::AddRelayCancelButtonPressed),
                            button(text("Ok").horizontal_alignment(Horizontal::Center),)
                                .width(Length::Fill)
                                .on_press(Message::AddRelaySubmit(relay_url.clone()))
                        ]
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill),
                    )
                    .max_width(CARD_MAX_WIDTH)
                    .on_close(Message::CloseAddRelayModal)
                    .into()
            })
            .backdrop(Message::CloseAddRelayModal)
            .on_esc(Message::CloseAddRelayModal)
            .into(),
            ModalState::Off => underlay.into(),
        }
    }

    fn open(&mut self) {
        *self = Self::AddRelay {
            relay_url: "".into(),
            is_invalid: false,
        }
    }

    fn close(&mut self) {
        *self = Self::Off
    }

    fn input_change(&mut self, input: String) {
        if let Self::AddRelay {
            relay_url,
            is_invalid,
        } = self
        {
            *relay_url = input;
            *is_invalid = false;
        }
    }

    fn error(&mut self) {
        if let Self::AddRelay { is_invalid, .. } = self {
            *is_invalid = true;
        }
    }
}

pub enum StepView {
    Welcome,
    Relays {
        relays_suggestion: Vec<nostr_sdk::Url>,
        relays_added: Vec<RelayRow>,
        add_relay_modal: ModalState,
    },
    DownloadEvents {
        relays: HashMap<nostr_sdk::Url, EventData>,
    },
    // Contacts,
    LoadingClient,
}
impl StepView {
    fn relays_view(conn: &mut BackEndConnection) -> Self {
        conn.send(net::Message::FetchRelays);

        let relays_suggestion: Vec<_> = RELAY_SUGGESTIONS
            .iter()
            .filter_map(|s| nostr_sdk::Url::parse(s).ok())
            .collect();

        Self::Relays {
            relays_suggestion,
            relays_added: vec![],
            add_relay_modal: ModalState::Off,
        }
    }
    fn download_events_view(conn: &mut BackEndConnection) -> Self {
        conn.send(net::Message::FetchRelays);
        Self::DownloadEvents {
            relays: HashMap::new(),
        }
    }
    fn loading_client(conn: &mut BackEndConnection) -> StepView {
        conn.send(net::Message::PrepareClient);
        Self::LoadingClient
    }
    fn get_step(&self) -> u8 {
        match self {
            StepView::Welcome => 1,
            StepView::Relays { .. } => 2,
            StepView::DownloadEvents { .. } => 3,
            StepView::LoadingClient => 4,
        }
    }
    const MAX_STEP: u8 = 3;
    fn make_dots(&self) -> Element<'static, Message> {
        let step = self.get_step();
        let mut dot_row = row![].spacing(5);

        for i in 0..Self::MAX_STEP {
            if i < step {
                dot_row = dot_row.push(solid_circle_icon().size(10));
            } else {
                dot_row = dot_row.push(regular_circle_icon().size(10));
            }
        }

        dot_row.width(Length::Shrink).into()
    }

    fn make_btns(&self) -> Element<'static, Message> {
        match self {
            StepView::Welcome => row![
                button("Cancel").on_press(Message::Exit),
                button("Next").on_press(Message::ToNextStep)
            ]
            .spacing(10)
            .into(),
            StepView::Relays { .. } => row![
                button("Back").on_press(Message::ToPreviousStep),
                button("Next").on_press(Message::ToNextStep)
            ]
            .spacing(10)
            .into(),
            StepView::DownloadEvents { .. } => button("Start").on_press(Message::ToApp).into(),
            // StepView::Contacts => row![
            //     button("Back").on_press(Message::ToPreviousStep),
            //     button("Start").on_press(Message::ToNextStep)
            // ]
            // .spacing(10)
            // .into(),
            Self::LoadingClient => text("").into(),
        }
    }

    fn make_step_buttons(&self) -> Element<'static, Message> {
        column![
            container(self.make_dots()).center_x().width(Length::Fill),
            container(self.make_btns()).center_x().width(Length::Fill)
        ]
        .spacing(5)
        .into()
    }
    pub fn view(&self) -> Element<Message> {
        let welcome_image = image::Image::new(image::Handle::from_memory(WELCOME_IMAGE));
        let relays_image = image::Image::new(image::Handle::from_memory(RELAYS_IMAGE));
        // let contacts_image = image::Image::new(image::Handle::from_memory(CONTACTS_IMAGE));

        match self {
            StepView::Welcome => {
                let title_1 = "NostrTalk";
                let text_1a = "Secure, encrypted chats on the NOSTR network";
                let content = column![
                    title(title_1)
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .center_x()
                        .center_y(),
                    container(
                        row![
                            container(welcome_image)
                                .max_width(WELCOME_IMAGE_MAX_WIDTH)
                                .height(Length::Fill),
                            container(text(text_1a).size(WELCOME_TEXT_SIZE))
                                .width(Length::Fixed(WELCOME_TEXT_WIDTH))
                                .height(Length::Fill)
                                .center_x()
                                .center_y(),
                        ]
                        .spacing(20)
                    )
                    .height(Length::FillPortion(4))
                    .width(Length::Fill)
                    .center_y()
                    .center_x(),
                    container(self.make_step_buttons()).height(Length::FillPortion(1))
                ]
                .spacing(30);

                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(style::Container::WelcomeBg1)
                    .into()
            }
            StepView::Relays {
                relays_added,
                relays_suggestion,
                add_relay_modal,
            } => {
                let title_2 = "Relays Setup";
                let text_2 = "Add relays to connect";
                let relays_suggestion =
                    relays_suggestion
                        .iter()
                        .fold(column![].spacing(5), |column, url| {
                            column.push(
                                container(
                                    row![
                                        text(url).size(20).width(Length::Fill),
                                        button("Add").on_press(Message::AddRelay(url.clone()))
                                    ]
                                    .align_items(Alignment::Center),
                                )
                                .width(Length::Fill)
                                .height(Length::Shrink),
                            )
                        });
                let relay_rows = relays_added
                    .iter()
                    .fold(column![].spacing(5), |col, relay| {
                        col.push(relay.relay_welcome().map(Message::RelayMessage))
                    });
                let add_other_btn = container(
                    button("Add Other")
                        .padding(10)
                        .on_press(Message::AddOtherPress),
                )
                .width(Length::Fill)
                .center_x()
                .center_y();
                let relays = container(common_scrollable(
                    column![relay_rows, relays_suggestion, add_other_btn]
                        .spacing(10)
                        .padding(20),
                ))
                .padding(5)
                .style(style::Container::Bordered)
                .width(Length::Fill)
                .height(Length::Fill);

                let content = column![
                    title(title_2)
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .center_x()
                        .center_y(),
                    container(
                        row![
                            container(relays_image)
                                .max_width(WELCOME_IMAGE_MAX_WIDTH)
                                .height(Length::Fill),
                            container(
                                column![
                                    container(text(text_2).size(WELCOME_TEXT_SIZE))
                                        .width(Length::Fill)
                                        .center_x(),
                                    relays
                                ]
                                .spacing(10)
                            )
                            .width(Length::Fixed(WELCOME_TEXT_WIDTH))
                            .height(Length::Fill),
                        ]
                        .spacing(20)
                    )
                    .height(Length::FillPortion(4))
                    .width(Length::Fill)
                    .center_y()
                    .center_x(),
                    container(column![self.make_step_buttons()].spacing(5))
                        .height(Length::FillPortion(1))
                ]
                .spacing(10);

                let underlay = container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(style::Container::WelcomeBg2);

                add_relay_modal.view(underlay).into()
            }
            StepView::DownloadEvents { relays } => {
                let title_1 = "Downloading events";

                let relays = relays
                    .iter()
                    .fold(column![].spacing(5), |col, (_url, ev_data)| {
                        col.push(ev_data.view())
                    });

                let relays_ct =
                    container(column![EventData::header(), common_scrollable(relays)].spacing(5))
                        .padding(10);

                let content = column![
                    title(title_1)
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .center_x()
                        .center_y(),
                    container(relays_ct)
                        .width(Length::Fill)
                        .height(Length::FillPortion(4))
                        .center_y()
                        .center_x(),
                    container(self.make_step_buttons()).height(Length::FillPortion(1))
                ]
                .spacing(10);

                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .style(style::Container::WelcomeBg3)
                    .into()
            }

            StepView::LoadingClient => inform_card("Loading", "Please wait...").into(),
        }
    }
}
pub struct State {
    pub step_view: StepView,
}
impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        match &self.step_view {
            StepView::Relays { relays_added, .. } => {
                let relay_subs: Vec<_> = relays_added
                    .iter()
                    .map(|r| r.subscription().map(Message::RelayMessage))
                    .collect();
                Subscription::batch(relay_subs)
            }
            _ => iced::Subscription::none(),
        }
    }
    pub fn new() -> Self {
        Self {
            step_view: StepView::Welcome,
        }
    }
    fn to_next_step(&mut self, conn: &mut BackEndConnection) {
        match &self.step_view {
            StepView::Welcome => {
                self.step_view = StepView::relays_view(conn);
            }
            StepView::Relays { relays_added, .. } => {
                if relays_added.is_empty() {
                    self.step_view = StepView::loading_client(conn)
                } else {
                    self.step_view = StepView::download_events_view(conn)
                }
            }
            StepView::DownloadEvents { .. } => {}
            StepView::LoadingClient => {}
        }
    }
    fn to_previous_step(&mut self, _conn: &mut BackEndConnection) {
        match &self.step_view {
            StepView::Welcome => {}
            StepView::Relays { .. } => self.step_view = StepView::Welcome,
            StepView::DownloadEvents { .. } => {}
            StepView::LoadingClient => {}
        }
    }

    pub fn update(&mut self, message: Message, conn: &mut BackEndConnection) {
        match message {
            Message::RelayMessage(msg) => {
                if let StepView::Relays { relays_added, .. } = &mut self.step_view {
                    if let Some(row) = relays_added.iter_mut().find(|r| r.id == msg.from) {
                        let _ = row.update(msg, conn);
                    }
                }
            }
            Message::Exit => (),
            Message::ToApp => (),
            Message::ToNextStep => self.to_next_step(conn),
            Message::ToPreviousStep => self.to_previous_step(conn),
            Message::AddRelay(relay_url) => {
                if let StepView::Relays { .. } = &mut self.step_view {
                    let db_relay = DbRelay::new(relay_url);
                    conn.send(net::Message::AddRelay(db_relay));
                }
            }
            Message::AddOtherPress => {
                if let StepView::Relays {
                    add_relay_modal, ..
                } = &mut self.step_view
                {
                    add_relay_modal.open();
                }
            }
            Message::AddRelayCancelButtonPressed => {
                if let StepView::Relays {
                    add_relay_modal, ..
                } = &mut self.step_view
                {
                    add_relay_modal.close();
                }
            }
            Message::AddRelayInputChange(input) => {
                if let StepView::Relays {
                    add_relay_modal, ..
                } = &mut self.step_view
                {
                    add_relay_modal.input_change(input);
                }
            }
            Message::AddRelaySubmit(relay_url) => {
                if let StepView::Relays {
                    add_relay_modal, ..
                } = &mut self.step_view
                {
                    match nostr_sdk::Url::parse(&relay_url) {
                        Ok(url) => {
                            let db_relay = DbRelay::new(url);
                            conn.send(net::Message::AddRelay(db_relay));
                            add_relay_modal.close();
                        }
                        Err(_e) => add_relay_modal.error(),
                    }
                }
            }
            Message::CloseAddRelayModal => {
                if let StepView::Relays {
                    add_relay_modal, ..
                } = &mut self.step_view
                {
                    add_relay_modal.close();
                }
            }
        }
    }
    pub fn backend_event(&mut self, event: net::events::Event, conn: &mut BackEndConnection) {
        match &mut self.step_view {
            StepView::DownloadEvents { relays } => {
                let event_str = event.to_string();
                match event {
                    net::events::Event::GotRelays(db_relays) => {
                        for db_r in &db_relays {
                            conn.send(net::Message::RequestEventsOf(db_r.clone()));
                        }
                    }
                    net::events::Event::RequestedEventsOf(db_relay) => {
                        relays.insert(db_relay.url.clone(), EventData::new(&db_relay.url));
                    }
                    net::events::Event::EndOfStoredEvents((relay_url, _sub_id)) => {
                        if let Some(ev_data) = relays.get_mut(&relay_url) {
                            ev_data.done();
                        }
                    }
                    net::events::Event::ReceivedDM { relay_url, .. }
                    | net::events::Event::ReceivedContactList { relay_url, .. }
                    | net::events::Event::UpdatedContactMetadata { relay_url, .. }
                    | net::events::Event::UpdatedUserProfileMeta { relay_url, .. } => {
                        if let Some(ev_data) = relays.get_mut(&relay_url) {
                            let event_str = format!("{}", event_str);
                            ev_data.add_event(&event_str);
                        }
                    }
                    _ => (),
                }
            }
            StepView::Relays {
                relays_added,
                relays_suggestion,
                ..
            } => match event {
                net::events::Event::GotRelays(mut db_relays) => {
                    for db_relay in db_relays.iter() {
                        relays_suggestion.retain(|url| url != &db_relay.url);
                    }
                    db_relays.sort_by(|a, b| a.url.cmp(&b.url));
                    *relays_added = db_relays
                        .into_iter()
                        .enumerate()
                        .map(|(idx, db_relay)| RelayRow::new(idx as i32, db_relay, conn))
                        .collect();
                }
                net::events::Event::RelayCreated(db_relay) => {
                    relays_suggestion.sort_by(|a, b| a.cmp(&b));
                    relays_added.sort_by(|a, b| a.db_relay.url.cmp(&b.db_relay.url));

                    relays_suggestion.retain(|url| url != &db_relay.url);
                    relays_added.push(RelayRow::new(relays_added.len() as i32, db_relay, conn));
                }
                net::events::Event::RelayDeleted(db_relay) => {
                    relays_suggestion.sort_by(|a, b| a.cmp(&b));
                    relays_added.sort_by(|a, b| a.db_relay.url.cmp(&b.db_relay.url));

                    relays_added.retain(|row| &row.db_relay.url != &db_relay.url);
                    relays_suggestion.push(db_relay.url);
                }
                other => {
                    relays_added
                        .iter_mut()
                        .for_each(|r| r.backend_event(other.clone(), conn));
                }
            },
            _ => (),
        }
    }
    pub fn view(&self) -> Element<Message> {
        self.step_view.view()
    }
}

#[derive(Debug, Clone)]
pub struct EventData {
    pub relay_url: nostr_sdk::Url,
    pub count: usize,
    pub last_event: String,
    pub is_done: bool,
}
impl EventData {
    pub fn get_description(&self) -> String {
        if self.is_done {
            format!("Got {} events", self.count)
        } else {
            format!("{}", self.last_event)
        }
    }
    pub fn new(relay_url: &nostr_sdk::Url) -> EventData {
        Self {
            relay_url: relay_url.to_owned(),
            count: 0,
            last_event: "".into(),
            is_done: false,
        }
    }
    pub fn add_event(&mut self, last_event: &str) {
        if !self.is_done {
            self.count += 1;
            self.last_event = last_event.into();
        }
    }
    pub fn done(&mut self) {
        self.is_done = true;
        self.last_event = "End of events".into();
    }
    pub fn header<Message: 'static>() -> Element<'static, Message> {
        container(
            row![
                text("Relay Url").size(24).width(Length::Fill),
                text("Last Event").size(24).width(Length::Fill),
                text("Events Added").size(24).width(Length::Fill),
            ]
            .align_items(Alignment::Center)
            .spacing(10),
        )
        .center_y()
        .into()
    }
    pub fn view<'a, Message: 'a>(&'a self) -> Element<'a, Message> {
        container(
            row![
                text(&self.relay_url).size(20).width(Length::Fill),
                text(&add_ellipsis_trunc(&self.last_event, 20))
                    .size(16)
                    .width(Length::Fill),
                text(self.count).size(20).width(Length::Fill),
            ]
            .align_items(Alignment::Center)
            .spacing(10),
        )
        .center_y()
        .into()
    }
}

fn _generate_random_ipv4() -> Ipv4Addr {
    let mut rng = thread_rng();
    let ip = Ipv4Addr::new(
        rng.gen_range(1..=255),
        rng.gen_range(0..=255),
        rng.gen_range(0..=255),
        rng.gen_range(0..=255),
    );
    ip
}

const WELCOME_IMAGE_MAX_WIDTH: f32 = 300.0;
const WELCOME_TEXT_SIZE: u16 = 24;
const WELCOME_TEXT_WIDTH: f32 = 400.0;
const CARD_MAX_WIDTH: f32 = 300.0;
