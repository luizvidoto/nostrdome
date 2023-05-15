use crate::components::text::title;
use crate::components::{common_scrollable, relay_row, RelayRow};
use crate::net::events::Event;
use crate::net::{self, BackEndConnection};
use crate::style;
use crate::widget::Element;
use iced::widget::{button, column, container, row, text};
use iced::Length;
use iced::{alignment, Subscription};
use iced_aw::{Card, Modal};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    CloseModal,
    UnderlayMessage(M),
    RelayRowMessage(relay_row::MessageWrapper),
}

#[derive(Debug, Clone)]
pub struct SendContactList {
    relays: Vec<RelayRow>,
}
impl SendContactList {
    pub fn new(conn: &mut BackEndConnection) -> Self {
        conn.send(net::Message::FetchRelays);
        Self { relays: vec![] }
    }
    pub fn subscription<'a, M: Clone + Debug + 'static>(&'a self) -> Subscription<CMessage<M>> {
        let relay_subs: Vec<_> = self
            .relays
            .iter()
            .map(|r| r.subscription().map(CMessage::RelayRowMessage))
            .collect();
        Subscription::batch(relay_subs)
    }
    pub fn backend_event(&mut self, event: Event, conn: &mut BackEndConnection) {
        self.relays
            .iter_mut()
            .for_each(|r| r.backend_event(event.clone(), conn));
        match event {
            Event::GotRelays(db_relays) => {
                self.relays = db_relays
                    .into_iter()
                    .enumerate()
                    .map(|(idx, db_relay)| RelayRow::new(idx as i32, db_relay, conn).with_mode())
                    .collect();
            }
            _ => (),
        }
    }
    pub fn update<'a, M: 'a + Clone + Debug>(
        &'a mut self,
        message: CMessage<M>,
        conn: &mut BackEndConnection,
    ) -> bool {
        match message {
            CMessage::RelayRowMessage(msg) => {
                if let Some(row) = self.relays.iter_mut().find(|r| r.id == msg.from) {
                    let _ = row.update(msg, conn);
                }
            }
            CMessage::UnderlayMessage(_) => (),
            CMessage::CloseModal => return true,
        }
        false
    }
    pub fn view<'a, M: 'a + Clone + Debug>(
        &'a self,
        underlay: impl Into<Element<'a, M>>,
    ) -> Element<'a, CMessage<M>> {
        let underlay_component = underlay.into().map(CMessage::UnderlayMessage);
        Modal::new(true, underlay_component, move || {
            let title = title("Send Contact List");
            // let send_to_all_btn = row![Space::with_width(Length::Fill), button("Send All").on_press(Message::SendContactListToAll)];
            let send_to_all_btn = text("");

            let header = container(title)
                .width(Length::Fill)
                .style(style::Container::Default)
                .center_y();

            let relay_list: Element<_> = self
                .relays
                .iter()
                .fold(column![send_to_all_btn].spacing(5), |col, relay_row| {
                    col.push(relay_row.modal_view().map(CMessage::RelayRowMessage))
                })
                .into();

            let modal_body = container(common_scrollable(relay_list)).width(Length::Fill);

            Card::new(header, modal_body)
                .foot(
                    row![button(
                        text("Cancel").horizontal_alignment(alignment::Horizontal::Center),
                    )
                    .width(Length::Fill)
                    .on_press(CMessage::CloseModal),]
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill),
                )
                .max_width(MODAL_WIDTH)
                .on_close(CMessage::CloseModal)
                .into()
        })
        .backdrop(CMessage::CloseModal)
        .on_esc(CMessage::CloseModal)
        .into()
    }
}

const MODAL_WIDTH: f32 = 400.0;
