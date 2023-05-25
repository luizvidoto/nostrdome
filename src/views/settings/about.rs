use crate::{
    components::{common_scrollable, text::title},
    consts::{BITCOIN_ADDRESS, GITHUB_REPO, LIGHTNING_ADDRESS, NOSTRTALK_VERSION, TT_LINK},
    icon::copy_icon,
    net::BackendEvent,
    style,
    utils::{format_btc_address, qr_code_handle},
    widget::Element,
};
use iced::widget::{button, column, container, image as iced_image, row, text, tooltip, Rule};
use iced::{clipboard, widget::image::Handle};
use iced::{Alignment, Command, Length};

#[derive(Debug, Clone)]
pub enum Message {
    BackEndEvent(BackendEvent),
    OpenTTLink,
    OpenGHLink,
    CopyQrCode(String),
}

#[derive(Debug, Clone)]
pub struct State {
    btc_qrcode_handle: Option<Handle>,
    lnd_qrcode_handle: Option<Handle>,
}
impl State {
    pub fn new() -> Self {
        Self {
            btc_qrcode_handle: qr_code_handle(BITCOIN_ADDRESS).ok(),
            lnd_qrcode_handle: qr_code_handle(LIGHTNING_ADDRESS).ok(),
        }
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::BackEndEvent(_ev) => (),
            Message::OpenTTLink => {
                if let Err(e) = webbrowser::open(TT_LINK) {
                    tracing::error!("Failed to open link: {}", e);
                }
            }
            Message::OpenGHLink => {
                if let Err(e) = webbrowser::open(GITHUB_REPO) {
                    tracing::error!("Failed to open link: {}", e);
                }
            }
            Message::CopyQrCode(content) => {
                return clipboard::write(content);
            }
        }
        Command::none()
    }

    pub fn view(&self) -> Element<Message> {
        let title = title("About");
        let version = text(format!("NostrTalk v{}", NOSTRTALK_VERSION))
            .size(18)
            .style(style::Text::Placeholder);

        let about_1 = text("NostrTalk is a messaging app that uses the NOSTR protocol.");

        let about_2 = text("This software is free and open source licensed under the MIT license.");

        let about_3 = text("NostrTalk is developed by ");
        let tt_link = button("@nickhntv")
            .padding(0)
            .style(style::Button::Link)
            .on_press(Message::OpenTTLink);
        let about_3_group = row![about_3, tt_link].align_items(Alignment::Center);

        let github_text = text("Source code available on ");
        let github_link = button("Github")
            .padding(0)
            .style(style::Button::Link)
            .on_press(Message::OpenGHLink);
        let github_group = row![github_text, github_link].align_items(Alignment::Center);
        let h_divider = container(Rule::horizontal(2))
            .padding(10)
            .width(Length::Fill);

        let donation_1 =
            text("If you like this software, please consider donating to the following addresses:");
        let donation_btc =
            make_donation_qrcode("Bitcoin", &self.btc_qrcode_handle, BITCOIN_ADDRESS);

        let donation_lnd = make_donation_qrcode(
            "Lightning Network",
            &self.lnd_qrcode_handle,
            LIGHTNING_ADDRESS,
        );

        let content = column![
            title,
            version,
            about_1,
            about_2,
            about_3_group,
            github_group,
            h_divider,
            donation_1,
            row![donation_btc, donation_lnd]
                .width(Length::Fill)
                .spacing(50),
        ]
        .spacing(10);
        container(common_scrollable(content))
            .width(Length::Fill)
            .into()
    }
}

fn make_donation_qrcode<'a>(
    name: &str,
    qr_code_handle: &Option<Handle>,
    qr_code_str: &str,
) -> Element<'a, Message> {
    let name = container(text(name).size(24))
        .width(Length::Fill)
        .center_x();

    let qrcode_image: Element<_> = if let Some(qr_code_handle) = qr_code_handle.as_ref() {
        iced_image(qr_code_handle.to_owned())
            .width(QR_CODE_WIDTH)
            .height(QR_CODE_HEIGHT)
            .into()
    } else {
        text("").into()
    };

    let qrcode_txt = container(
        text(format_btc_address(qr_code_str))
            .size(18)
            .style(style::Text::Placeholder),
    );
    //quando clica no botao, aparece uma tooltip falando "copied!"
    let copy_btn = tooltip(
        button(copy_icon())
            .on_press(Message::CopyQrCode(qr_code_str.to_owned()))
            .style(style::Button::Primary),
        "Copy",
        tooltip::Position::Top,
    )
    .style(style::Container::TooltipBg);

    let qrcode_txt_group = row![qrcode_txt, copy_btn]
        .align_items(Alignment::Center)
        .spacing(5);

    let content = column![name, qrcode_image, qrcode_txt_group]
        .spacing(5)
        .align_items(Alignment::Center);

    container(content).width(Length::Fill).into()
}

const QR_CODE_WIDTH: f32 = 220.0;
const QR_CODE_HEIGHT: f32 = 220.0;
