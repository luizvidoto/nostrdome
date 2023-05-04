use iced::alignment;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Command, Length, Subscription};
use iced_aw::{Card, Modal};
use nostr_sdk::{Metadata, Tag};

use crate::components::text::title;
use crate::components::text_input_group::TextInputGroup;
use crate::components::{file_importer, FileImporter};
use crate::db::{DbContact, DbRelay};
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::types::UncheckedEvent;
use crate::utils::json_reader;

use crate::widget::{Button, Element};

use self::relay_row_modal::RelayRowModal;

mod about;
mod account;
pub mod appearance;
mod backup;
mod contacts;
mod network;

#[derive(Debug, Clone)]
pub enum Message {
    RelayRowMessage(relay_row_modal::Message),
    AccountMessage(account::Message),
    AppearanceMessage(appearance::Message),
    NetworkMessage(network::Message),
    BackupMessage(backup::Message),
    ContactsMessage(contacts::Message),
    AboutMessage(about::Message),
    MenuAccountPress,
    MenuAppearancePress,
    MenuNetworkPress,
    MenuBackupPress,
    MenuContactsPress,
    MenuAboutPress,
    LogoutPress,
    NavEscPress,

    // Modal Messages
    ModalPetNameInputChange(String),
    ModalPubKeyInputChange(String),
    ModalRecRelayInputChange(String),
    SubmitContact,
    SaveImportedContacts(Vec<DbContact>),
    FileImporterMessage(file_importer::Message),
    CloseImportContactModal,
    CloseAddContactModal,
    CloseSendContactsModal,
    SendContactListToAll,
}

#[derive(Debug)]
#[repr(u8)]
pub enum MenuState {
    Account { state: account::State } = 0,
    Appearance { state: appearance::State } = 1,
    Network { state: network::State } = 2,
    Backup { state: backup::State } = 3,
    Contacts { state: contacts::State } = 4,
    About { state: about::State } = 10,
}

impl MenuState {
    pub fn is_same_type(&self, other: u8) -> bool {
        match (self, other) {
            (MenuState::Account { .. }, 0) => true,
            (MenuState::Appearance { .. }, 1) => true,
            (MenuState::Network { .. }, 2) => true,
            (MenuState::Backup { .. }, 3) => true,
            (MenuState::Contacts { .. }, 4) => true,
            (MenuState::About { .. }, 10) => true,
            _ => false,
        }
    }
    fn account(_db_conn: &mut BackEndConnection) -> Self {
        let profile = Metadata::new();
        // conn.send(message)
        Self::Account {
            state: account::State::new(profile),
        }
    }
    fn appearance(selected_theme: Option<style::Theme>) -> Self {
        Self::Appearance {
            state: appearance::State::new(selected_theme),
        }
    }
    pub fn network(db_conn: &mut BackEndConnection) -> Self {
        Self::Network {
            state: network::State::new(db_conn),
        }
    }
    pub fn about(_db_conn: &mut BackEndConnection) -> Self {
        Self::About {
            state: about::State::new(),
        }
    }
    pub fn contacts(db_conn: &mut BackEndConnection) -> Self {
        Self::Contacts {
            state: contacts::State::new(db_conn),
        }
    }
    fn backup() -> Self {
        Self::Backup {
            state: backup::State::default(),
        }
    }
    pub fn new(db_conn: &mut BackEndConnection) -> Self {
        Self::account(db_conn)
    }
    pub fn view(&self) -> Element<Message> {
        match self {
            Self::Account { state } => state.view().map(Message::AccountMessage),
            Self::Appearance { state } => state.view().map(Message::AppearanceMessage),
            Self::Network { state } => state.view().map(Message::NetworkMessage),
            Self::Backup { state } => state.view().map(Message::BackupMessage),
            Self::Contacts { state } => state.view().map(Message::ContactsMessage),
            Self::About { state } => state.view().map(Message::AboutMessage),
        }
    }
}

#[derive(Debug)]
pub struct Settings {
    menu_state: MenuState,
    modal_state: ModalState,
}
impl Settings {
    pub fn subscription(&self) -> Subscription<Message> {
        match &self.menu_state {
            MenuState::Network { state } => state.subscription().map(Message::NetworkMessage),
            _ => Subscription::none(),
        }
    }
    pub fn new(db_conn: &mut BackEndConnection) -> Self {
        Self {
            menu_state: MenuState::new(db_conn),
            modal_state: ModalState::Off,
        }
    }
    pub fn contacts(db_conn: &mut BackEndConnection) -> Self {
        Self {
            menu_state: MenuState::contacts(db_conn),
            modal_state: ModalState::Off,
        }
    }
    pub fn network(db_conn: &mut BackEndConnection) -> Self {
        Self {
            menu_state: MenuState::network(db_conn),
            modal_state: ModalState::Off,
        }
    }
    pub fn about(db_conn: &mut BackEndConnection) -> Self {
        Self {
            menu_state: MenuState::about(db_conn),
            modal_state: ModalState::Off,
        }
    }

    pub fn backend_event(
        &mut self,
        event: Event,
        conn: &mut BackEndConnection,
    ) -> Command<Message> {
        self.modal_state.backend_event(event.clone(), conn);

        match &mut self.menu_state {
            MenuState::About { state } => state.update(about::Message::BackEndEvent(event)),
            MenuState::Account { state } => state.update(account::Message::BackEndEvent(event)),
            MenuState::Appearance { state } => {
                state.update(appearance::Message::BackEndEvent(event))
            }
            MenuState::Network { state } => {
                return state
                    .update(network::Message::BackEndEvent(event), conn)
                    .map(Message::NetworkMessage);
            }
            MenuState::Backup { state } => state.update(backup::Message::BackEndEvent(event)),
            MenuState::Contacts { state } => {
                let _ = state.update(contacts::Message::BackEndEvent(event), conn);
            }
        }

        Command::none()
    }
    pub fn update(
        &mut self,
        message: Message,
        conn: &mut BackEndConnection,
        selected_theme: Option<style::Theme>,
    ) -> Command<Message> {
        match message {
            Message::AccountMessage(msg) => {
                if let MenuState::Account { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::AppearanceMessage(msg) => {
                if let MenuState::Appearance { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::NetworkMessage(msg) => {
                if let MenuState::Network { state } = &mut self.menu_state {
                    return state.update(msg, conn).map(Message::NetworkMessage);
                }
            }
            Message::BackupMessage(msg) => {
                if let MenuState::Backup { state } = &mut self.menu_state {
                    state.update(msg);
                }
            }
            Message::ContactsMessage(msg) => {
                if let MenuState::Contacts { state } = &mut self.menu_state {
                    match msg {
                        contacts::Message::OpenAddContactModal => {
                            self.modal_state = ModalState::add_contact(None);
                        }
                        contacts::Message::OpenImportContactModal => {
                            self.modal_state = ModalState::import_contacts();
                        }
                        contacts::Message::OpenSendContactModal => {
                            self.modal_state = ModalState::load_send_contacts(conn);
                        }
                        other => {
                            if let Some(received_msg) = state.update(other, conn) {
                                match received_msg {
                                    contacts::Message::OpenEditContactModal(contact) => {
                                        self.modal_state = ModalState::add_contact(Some(contact))
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                }
            }
            Message::NavEscPress => (),
            Message::MenuAccountPress => {
                self.menu_state = MenuState::account(conn);
            }
            Message::MenuAppearancePress => {
                self.menu_state = MenuState::appearance(selected_theme);
            }
            Message::MenuNetworkPress => {
                self.menu_state = MenuState::network(conn);
            }
            Message::MenuBackupPress => {
                self.menu_state = MenuState::backup();
            }
            Message::MenuContactsPress => {
                self.menu_state = MenuState::contacts(conn);
            }
            Message::MenuAboutPress => {
                self.menu_state = MenuState::about(conn);
            }
            Message::LogoutPress => {
                conn.send(net::Message::Logout);
            }
            other => return self.modal_state.update(other, conn),
        }
        Command::none()
    }

    pub fn view(&self) -> Element<Message> {
        let account_btn =
            create_menu_button("Account", &self.menu_state, 0, Message::MenuAccountPress);
        let appearance_btn = create_menu_button(
            "Appearance",
            &self.menu_state,
            1,
            Message::MenuAppearancePress,
        );
        let network_btn =
            create_menu_button("Network", &self.menu_state, 2, Message::MenuNetworkPress);
        let backup_btn =
            create_menu_button("Backup", &self.menu_state, 3, Message::MenuBackupPress);
        let contacts_btn =
            create_menu_button("Contacts", &self.menu_state, 4, Message::MenuContactsPress);
        let about_btn = create_menu_button("About", &self.menu_state, 10, Message::MenuAboutPress);
        let logout_btn = button("Logout")
            .padding(10)
            .on_press(Message::LogoutPress)
            .style(style::Button::MenuBtn);
        let esc_btn = button("Esc")
            .padding(10)
            .on_press(Message::NavEscPress)
            .style(style::Button::MenuBtn);
        let spacer = iced::widget::horizontal_space(Length::Fixed(3.0));
        let menubar = container(
            column![
                esc_btn,
                spacer,
                account_btn,
                appearance_btn,
                network_btn,
                backup_btn,
                contacts_btn,
                about_btn,
                Space::with_height(Length::Fill),
                logout_btn
            ]
            .spacing(3),
        )
        .height(Length::Fill)
        .width(Length::FillPortion(1))
        .padding([10, 5]);

        let view_ct = container(self.menu_state.view())
            .style(style::Container::ChatContainer)
            .padding([20, 20, 0, 20])
            .height(Length::Fill)
            .width(Length::FillPortion(3));

        let content: Element<_> = container(
            row![menubar, view_ct]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .style(style::Container::Default)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        self.modal_state.view(content)
    }
}

#[derive(Debug, Clone)]
enum ModalState {
    ContactDetails {
        modal_petname_input: String,
        modal_pubkey_input: String,
        modal_rec_relay_input: String,
        is_edit: bool,
        is_pub_invalid: bool,
        is_relay_invalid: bool,
    },
    ImportList {
        imported_contacts: Vec<DbContact>,
        file_importer: FileImporter,
    },
    SendContactList {
        relays: Vec<relay_row_modal::RelayRowModal>,
        send_contacts_to_relay: Option<DbRelay>,
    },
    Off,
}

impl ModalState {
    pub fn backend_event(&mut self, event: Event, conn: &mut BackEndConnection) {
        if let ModalState::SendContactList {
            relays,
            send_contacts_to_relay,
        } = self
        {
            match event {
                Event::GotContacts(contacts) => {
                    if let Some(db_relay) = &send_contacts_to_relay {
                        conn.send(net::Message::SendContactListToRelay((
                            db_relay.clone(),
                            contacts,
                        )));
                    }
                }
                Event::GotRelays(db_relays) => {
                    *relays = db_relays
                        .iter()
                        .map(|db_relay| RelayRowModal::new(db_relay))
                        .collect();
                }
                Event::UpdateWithRelayResponse { relay_response, .. } => {
                    if let Some(relay_row) = relays
                        .iter_mut()
                        .find(|row| row.db_relay.url == relay_response.relay_url)
                    {
                        relay_row.success();
                        *send_contacts_to_relay = None;
                    }
                }
                _ => (),
            }
        }
    }
    pub fn load_send_contacts(db_conn: &mut BackEndConnection) -> Self {
        db_conn.send(net::Message::FetchRelays);
        Self::SendContactList {
            relays: vec![],
            send_contacts_to_relay: None,
        }
    }
    pub fn add_contact(contact: Option<DbContact>) -> Self {
        let (petname, pubkey, rec_relay, is_edit) = match contact {
            Some(c) => (
                c.get_petname().unwrap_or_else(|| "".into()),
                c.pubkey().to_string(),
                c.get_relay_url()
                    .map(|url| url.to_string())
                    .unwrap_or("".into()),
                true,
            ),
            None => ("".into(), "".into(), "".into(), false),
        };

        Self::ContactDetails {
            modal_petname_input: petname,
            modal_pubkey_input: pubkey,
            modal_rec_relay_input: rec_relay,
            is_edit,
            is_pub_invalid: false,
            is_relay_invalid: false,
        }
    }
    pub fn import_contacts() -> Self {
        Self::ImportList {
            imported_contacts: vec![],
            file_importer: FileImporter::new("/path/to/contacts.json")
                .file_filter("JSON File", &["json"]),
        }
    }
    pub fn update(
        &mut self,
        message: Message,
        db_conn: &mut BackEndConnection,
    ) -> Command<Message> {
        match message {
            Message::CloseSendContactsModal => *self = Self::Off,
            Message::CloseAddContactModal => *self = Self::Off,
            Message::CloseImportContactModal => *self = Self::Off,
            Message::SendContactListToAll => {
                println!("Send contacts to all relays");
            }
            Message::RelayRowMessage(r_msg) => match r_msg {
                relay_row_modal::Message::SendContactListToRelay(db_relay) => {
                    db_conn.send(net::Message::FetchContacts);
                    if let ModalState::SendContactList {
                        relays,
                        send_contacts_to_relay,
                    } = self
                    {
                        if let Some(relay_row) = relays
                            .iter_mut()
                            .find(|row| row.db_relay.url == db_relay.url)
                        {
                            *send_contacts_to_relay = Some(db_relay.clone());
                            relay_row.loading();
                        }
                    }
                }
            },
            Message::SubmitContact => {
                if let ModalState::ContactDetails {
                    is_pub_invalid,
                    is_relay_invalid,
                    modal_petname_input: petname,
                    modal_pubkey_input: pubkey,
                    modal_rec_relay_input: rec_relay,
                    is_edit,
                } = self
                {
                    match DbContact::from_submit(&pubkey, &petname, &rec_relay) {
                        Ok(db_contact) => {
                            db_conn.send(match is_edit {
                                true => net::Message::UpdateContact(db_contact),
                                false => net::Message::AddContact(db_contact),
                            });
                            *self = ModalState::Off;
                        }
                        Err(e) => {
                            tracing::error!("Error: {:?}", e);
                            match e {
                                crate::db::DbContactError::InvalidPublicKey => {
                                    *is_pub_invalid = true;
                                }
                                crate::db::DbContactError::InvalidRelayUrl(_) => {
                                    *is_relay_invalid = true;
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
            Message::SaveImportedContacts(imported_contacts) => {
                db_conn.send(net::Message::ImportContacts(imported_contacts));
                *self = ModalState::Off;
            }
            Message::FileImporterMessage(msg) => {
                if let ModalState::ImportList {
                    imported_contacts,
                    file_importer,
                } = self
                {
                    if let Some(returned_msg) = file_importer.update(msg) {
                        handle_file_importer_message(returned_msg, imported_contacts);
                    }
                }
            }
            Message::ModalPetNameInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_petname_input,
                    ..
                } = self
                {
                    *modal_petname_input = text;
                }
            }
            Message::ModalPubKeyInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_pubkey_input,
                    is_pub_invalid,
                    ..
                } = self
                {
                    *modal_pubkey_input = text;
                    *is_pub_invalid = false;
                }
            }
            Message::ModalRecRelayInputChange(text) => {
                if let ModalState::ContactDetails {
                    modal_rec_relay_input,
                    is_relay_invalid,
                    ..
                } = self
                {
                    *modal_rec_relay_input = text;
                    *is_relay_invalid = false;
                }
            }
            _ => (),
        }

        Command::none()
    }
    pub fn view<'a>(&'a self, underlay: Element<'a, Message>) -> Element<'a, Message> {
        let view: Element<_> = match self {
            ModalState::SendContactList { relays, .. } => Modal::new(true, underlay, move || {
                let title = title("Send Contact List");
                let send_to_all_btn = button("Send All").on_press(Message::SendContactListToAll);
                let header = container(title)
                    .width(Length::Fill)
                    .style(style::Container::Default)
                    .center_y();

                let relay_list: Element<_> = relays
                    .iter()
                    .fold(
                        column![row![Space::with_width(Length::Fill), send_to_all_btn]].spacing(5),
                        |col, relay_row| col.push(relay_row.view().map(Message::RelayRowMessage)),
                    )
                    .into();

                let modal_body = container(scrollable(relay_list))
                    .width(Length::Fill)
                    .style(style::Container::Default);

                Card::new(header, modal_body)
                    .foot(
                        row![button(
                            text("Cancel").horizontal_alignment(alignment::Horizontal::Center),
                        )
                        .width(Length::Fill)
                        .on_press(Message::CloseSendContactsModal),]
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill),
                    )
                    .max_width(ADD_CONTACT_MODAL_WIDTH)
                    .on_close(Message::CloseSendContactsModal)
                    .into()
            })
            .backdrop(Message::CloseSendContactsModal)
            .on_esc(Message::CloseSendContactsModal)
            .into(),
            ModalState::ContactDetails {
                modal_petname_input,
                modal_pubkey_input,
                modal_rec_relay_input,
                is_edit,
                is_pub_invalid,
                is_relay_invalid,
            } => Modal::new(true, underlay, move || {
                let add_relay_input = TextInputGroup::new(
                    "Contact Name",
                    modal_petname_input,
                    Message::ModalPetNameInputChange,
                )
                .placeholder("");

                let mut add_pubkey_input = TextInputGroup::new(
                    "Contact PubKey",
                    modal_pubkey_input,
                    Message::ModalPubKeyInputChange,
                )
                .placeholder("");

                if *is_pub_invalid {
                    add_pubkey_input = add_pubkey_input.invalid("Invalid Public Key");
                }

                let mut add_rec_relay_input = TextInputGroup::new(
                    "Recommended Relay",
                    modal_rec_relay_input,
                    Message::ModalRecRelayInputChange,
                )
                .placeholder("ws://my-relay.com");

                if *is_relay_invalid {
                    add_rec_relay_input = add_rec_relay_input.invalid("Invalid Relay URL");
                }

                let modal_body: Element<_> = column![
                    add_relay_input.build(),
                    add_pubkey_input.build(),
                    add_rec_relay_input.build()
                ]
                .spacing(4)
                .into();
                let title = match is_edit {
                    true => text("Edit Contact"),
                    false => text("Add Contact"),
                };
                Card::new(title, modal_body)
                    .foot(
                        row![
                            button(
                                text("Cancel").horizontal_alignment(alignment::Horizontal::Center),
                            )
                            .width(Length::Fill)
                            .on_press(Message::CloseAddContactModal),
                            button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                                .width(Length::Fill)
                                .on_press(Message::SubmitContact)
                        ]
                        .spacing(10)
                        .padding(5)
                        .width(Length::Fill),
                    )
                    .max_width(ADD_CONTACT_MODAL_WIDTH)
                    .on_close(Message::CloseAddContactModal)
                    .into()
            })
            .backdrop(Message::CloseAddContactModal)
            .on_esc(Message::CloseAddContactModal)
            .into(),
            ModalState::ImportList {
                imported_contacts,
                file_importer,
            } => Modal::new(true, underlay, || {
                let importer_cp = file_importer.view().map(Message::FileImporterMessage);
                let found_contacts_txt = match imported_contacts.len() {
                    0 => text(""),
                    n => text(format!("Found contacts: {}", n)),
                };
                let stats_row = row![found_contacts_txt];

                let card_header = text("Import Contacts");
                let card_body = column![importer_cp, stats_row].spacing(4);
                let card_footer = row![
                    button(text("Cancel").horizontal_alignment(alignment::Horizontal::Center),)
                        .width(Length::Fill)
                        .on_press(Message::CloseImportContactModal),
                    button(text("Ok").horizontal_alignment(alignment::Horizontal::Center),)
                        .width(Length::Fill)
                        .on_press(Message::SaveImportedContacts(imported_contacts.clone()))
                ]
                .spacing(10)
                .padding(5)
                .width(Length::Fill);

                let card: Element<_> = Card::new(card_header, card_body)
                    .foot(card_footer)
                    .max_width(IMPORT_MODAL_WIDTH)
                    .on_close(Message::CloseImportContactModal)
                    .style(crate::style::Card::Default)
                    .into();

                card
            })
            .backdrop(Message::CloseImportContactModal)
            .on_esc(Message::CloseImportContactModal)
            .into(),
            ModalState::Off => underlay.into(),
        };

        view
    }
}

fn create_menu_button<'a>(
    label: &'a str,
    current_menu_state: &MenuState,
    target_menu_type: u8,
    msg: Message,
) -> Button<'a, Message> {
    let is_active = current_menu_state.is_same_type(target_menu_type);

    let style = if is_active {
        style::Button::ActiveMenuBtn
    } else {
        style::Button::MenuBtn
    };

    button(label)
        .style(style)
        .width(Length::Fill)
        .padding(10)
        .on_press(msg)
}

fn handle_file_importer_message(
    returned_msg: file_importer::Message,
    imported_contacts: &mut Vec<DbContact>,
) {
    if let file_importer::Message::OnChoose(path) = returned_msg {
        match json_reader::<String, UncheckedEvent>(path) {
            Ok(contact_event) => {
                if let nostr_sdk::event::Kind::ContactList = contact_event.kind {
                    update_imported_contacts(&contact_event.tags, imported_contacts);
                }
            }
            Err(e) => tracing::error!("{}", e),
        }
    }
}

fn update_imported_contacts(tags: &[Tag], imported_contacts: &mut Vec<DbContact>) {
    let (oks, errs): (Vec<_>, Vec<_>) = tags
        .iter()
        .map(DbContact::from_tag)
        .partition(Result::is_ok);

    let errors: Vec<_> = errs.into_iter().map(Result::unwrap_err).collect();

    for e in errors {
        tracing::error!("{}", e);
    }

    *imported_contacts = oks.into_iter().map(Result::unwrap).collect();
}

const ADD_CONTACT_MODAL_WIDTH: f32 = 400.0;
const IMPORT_MODAL_WIDTH: f32 = 400.0;

mod relay_row_modal {
    use crate::{db::DbRelay, style, widget::Element};
    use iced::{
        widget::{button, container, row, text, Space},
        Length,
    };

    #[derive(Debug, Clone)]
    pub enum Message {
        SendContactListToRelay(DbRelay),
    }

    #[derive(Debug, Clone)]
    pub enum RelayRowState {
        Idle,
        Loading,
        Success,
        Error(String),
    }
    #[derive(Debug, Clone)]
    pub struct RelayRowModal {
        pub state: RelayRowState,
        pub db_relay: DbRelay,
    }
    impl RelayRowModal {
        pub fn new(db_relay: &DbRelay) -> Self {
            Self {
                state: RelayRowState::Idle,
                db_relay: db_relay.to_owned(),
            }
        }
        pub fn loading(&mut self) {
            self.state = RelayRowState::Loading;
        }
        pub fn success(&mut self) {
            self.state = RelayRowState::Success;
        }
        pub fn view(&self) -> Element<Message> {
            let button_or_checkmark: Element<_> = match self.state {
                RelayRowState::Idle => button("Send")
                    .style(style::Button::Primary)
                    .on_press(Message::SendContactListToRelay(self.db_relay.clone()))
                    .into(),
                RelayRowState::Loading => button("...").style(style::Button::Primary).into(),
                RelayRowState::Success => text("Sent!").into(),
                RelayRowState::Error(_) => text("Error").into(),
            };

            container(row![
                text(&self.db_relay.url),
                Space::with_width(Length::Fill),
                button_or_checkmark
            ])
            .center_y()
            .into()
        }
    }
}
