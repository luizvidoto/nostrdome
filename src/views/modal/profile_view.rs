use crate::components::text::title;
use crate::db::DbContact;
use crate::net::BackEndConnection;
use crate::widget::Element;
use iced::alignment;
use iced::widget::{button, column, container, row, text};
use iced::Length;
use iced_aw::{Card, Modal};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum CMessage<M: Clone + Debug> {
    CloseModal,
    UnderlayMessage(M),
}

#[derive(Debug, Clone)]
pub struct ProfileView {
    contact: DbContact,
}
impl ProfileView {
    pub fn new(contact: DbContact) -> Self {
        Self { contact }
    }
    pub fn update<'a, M: 'a + Clone + Debug>(
        &'a mut self,
        message: CMessage<M>,
        conn: &mut BackEndConnection,
    ) -> bool {
        match message {
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
            let title = title("Profile");
            let header = container(title).width(Length::Fill).center_y();
            let card_body: Element<_> =
                if let Some(profile_cache) = self.contact.get_profile_cache() {
                    let profile_meta = profile_cache.metadata;
                    let mut content = column![].spacing(5);
                    if let Some(name) = profile_meta.name {
                        content = content.push(column![text("name"), text(name)].spacing(5));
                    }
                    if let Some(display_name) = profile_meta.display_name {
                        content = content
                            .push(column![text("display_name"), text(display_name)].spacing(5));
                    }
                    if let Some(picture_url) = profile_meta.picture {
                        content = content
                            .push(column![text("picture_url"), text(picture_url)].spacing(5));
                    }
                    if let Some(about) = profile_meta.about {
                        content = content.push(column![text("about"), text(about)].spacing(5));
                    }
                    if let Some(website) = profile_meta.website {
                        content = content.push(column![text("website"), text(website)].spacing(5));
                    }
                    if let Some(banner_url) = profile_meta.banner {
                        content =
                            content.push(column![text("banner_url"), text(banner_url)].spacing(5));
                    }
                    if let Some(nip05) = profile_meta.nip05 {
                        content = content.push(column![text("nip05"), text(nip05)].spacing(5));
                    }
                    if let Some(lud06) = profile_meta.lud06 {
                        content = content.push(column![text("lud06"), text(lud06)].spacing(5));
                    }
                    if let Some(lud16) = profile_meta.lud16 {
                        content = content.push(column![text("lud16"), text(lud16)].spacing(5));
                    }
                    content.into()
                } else {
                    text("No profile data found").into()
                };
            let card_body: Element<_> = container(card_body)
                .width(Length::Fill)
                .center_y()
                .center_x()
                .padding(10)
                .into();

            Card::new(header, card_body)
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
