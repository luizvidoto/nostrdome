use crate::error::Error;
use async_trait::async_trait;
use iced::futures::channel::mpsc;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

// fn process_message_queue(&mut self) {
//     if let Ok(mut sender) = self.sender().lock() {
//         if let Some(sender) = sender.as_mut() {
//             if let Ok(mut queue) = self.message_queue().lock() {
//                 while let Some(message) = queue.pop() {
//                     if let Err(e) = sender.unbounded_send(message).map_err(|e| e.to_string()) {
//                         tracing::error!("{}", e);
//                     }
//                 }
//             }
//         }
//     }
// }
#[async_trait]
pub trait Connection<M: Debug + Clone + Send + 'static> {
    fn new() -> Self;

    fn with_channel(&mut self, sender: mpsc::UnboundedSender<M>);

    fn sender(&self) -> Arc<Mutex<Option<mpsc::UnboundedSender<M>>>>;

    fn message_queue(&self) -> Arc<Mutex<Vec<M>>>;

    fn process_message_queue(&self);

    fn send(&mut self, message: M) {
        if let Err(e) = self.try_send_internal(message) {
            tracing::error!("{}", e);
        }
    }

    fn try_send(&mut self, message: M) -> Result<(), Error> {
        self.try_send_internal(message)
    }

    fn try_send_internal(&mut self, message: M) -> Result<(), Error> {
        let sender = self.sender();
        let mut sender_guard = sender
            .lock()
            .map_err(|_| Error::TrySendError(format!("{:?}", message)))?;
        let sender = match sender_guard.as_mut() {
            Some(sender) => sender,
            None => {
                let message_queue = self.message_queue();
                let mut queue_guard = message_queue
                    .lock()
                    .map_err(|_| Error::TrySendError(format!("{:?}", message)))?;
                queue_guard.push(message);
                return Ok(());
            }
        };

        if let Err(e) = sender.unbounded_send(message) {
            tracing::error!("{}", e);
            return Err(Error::TrySendError("".into()));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BackEndConnection<M: Debug + Clone + Send + 'static> {
    sender: Arc<Mutex<Option<mpsc::UnboundedSender<M>>>>,
    message_queue: Arc<Mutex<Vec<M>>>,
}

impl<M: Debug + Clone + Send + 'static> Connection<M> for BackEndConnection<M> {
    fn new() -> Self {
        Self {
            sender: Arc::new(Mutex::new(None)),
            message_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_channel(&mut self, sender: mpsc::UnboundedSender<M>) {
        if let Ok(mut sender_lock) = self.sender.lock() {
            *sender_lock = Some(sender);
        } else {
            tracing::error!("Failed to lock the sender");
        }
    }

    fn sender(&self) -> Arc<Mutex<Option<mpsc::UnboundedSender<M>>>> {
        self.sender.clone()
    }

    fn message_queue(&self) -> Arc<Mutex<Vec<M>>> {
        self.message_queue.clone()
    }

    fn process_message_queue(&self) {
        if let Ok(mut sender_lock) = self.sender.lock() {
            if let Some(sender) = sender_lock.as_mut() {
                if let Ok(mut message_queue_lock) = self.message_queue.lock() {
                    while let Some(message) = message_queue_lock.pop() {
                        if let Err(e) = sender.unbounded_send(message).map_err(|e| e.to_string()) {
                            tracing::error!("{}", e);
                            break;
                        }
                    }
                } else {
                    tracing::error!("Failed to lock the message queue");
                }
            }
        } else {
            tracing::error!("Failed to lock the sender");
        }
    }
}
