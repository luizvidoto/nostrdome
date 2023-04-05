use futures::channel::mpsc::UnboundedSender;
use iced::futures::{channel::mpsc, StreamExt};
use iced::{subscription, Subscription};

#[derive(Debug, Clone)]
pub enum Message {
    Ready(UnboundedSender<String>),
    ProgressEvent(String),
    Command(CmdMessage),
}
#[derive(Debug, Clone)]
pub enum CmdMessage {
    Finished,
}

pub enum ProgressState {
    Starting,
    Ready(mpsc::UnboundedReceiver<String>),
}

pub fn bind() -> Subscription<Message> {
    struct Progress;

    subscription::unfold(
        std::any::TypeId::of::<Progress>(),
        ProgressState::Starting,
        |state| async move {
            match state {
                ProgressState::Starting => {
                    let (sender, receiver) = mpsc::unbounded();

                    (Some(Message::Ready(sender)), ProgressState::Ready(receiver))
                }
                ProgressState::Ready(mut progress_receiver) => {
                    let received = progress_receiver.next().await;
                    if let Some(progress) = received {
                        if progress.contains("Finished") {
                            return (
                                Some(Message::Command(CmdMessage::Finished)),
                                ProgressState::Ready(progress_receiver),
                            );
                        }

                        return (
                            Some(Message::ProgressEvent(progress)),
                            ProgressState::Ready(progress_receiver),
                        );
                    }

                    (None, ProgressState::Ready(progress_receiver))
                }
            }
        },
    )
}
