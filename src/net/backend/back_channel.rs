use iced::futures::channel::mpsc;
use std::fmt::Debug;

use super::ToBackend;

#[derive(Debug, Clone)]
pub struct BackEndConnection(mpsc::UnboundedSender<ToBackend>);
impl BackEndConnection {
    pub fn new(sender: mpsc::UnboundedSender<ToBackend>) -> Self {
        Self(sender)
    }
    pub fn send(&mut self, input: ToBackend) {
        if let Err(e) = self.0.unbounded_send(input).map_err(|e| e.to_string()) {
            tracing::error!("{}", e);
        }
    }
}
