use crate::error::Error;
use async_trait::async_trait;
use iced::futures::channel::mpsc;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

#[async_trait]
pub trait Connection<M: Debug + Clone + Send + 'static> {
    fn new() -> Self;
    fn with_channel(&mut self, sender: mpsc::UnboundedSender<M>);

    fn sender(&self) -> Arc<Mutex<Option<mpsc::UnboundedSender<M>>>>;
    fn message_queue(&self) -> Arc<Mutex<Vec<M>>>;

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

    fn process_message_queue(&self);

    fn send(&mut self, message: M) {
        if let Ok(mut sender) = self.sender().lock() {
            if let Some(sender) = sender.as_mut() {
                if let Err(e) = sender.unbounded_send(message).map_err(|e| e.to_string()) {
                    tracing::error!("{}", e);
                }
            } else {
                if let Ok(mut queue) = self.message_queue().lock() {
                    queue.push(message);
                } else {
                    tracing::error!("Failed to lock the message queue");
                }
            }
        } else {
            tracing::error!("Failed to lock the sender");
        }
    }

    fn try_send(&mut self, message: M) -> Result<(), Error> {
        if let Ok(mut sender) = self.sender().lock() {
            if let Some(sender) = sender.as_mut() {
                if let Err(e) = sender.unbounded_send(message) {
                    tracing::error!("{}", e);
                }
                Ok(())
            } else {
                if let Ok(mut queue) = self.message_queue().lock() {
                    queue.push(message);
                    Ok(())
                } else {
                    Err(Error::TrySendError(format!("{:?}", message)))
                }
            }
        } else {
            Err(Error::TrySendError(format!("{:?}", message)))
        }
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
        *self.sender.lock().unwrap() = Some(sender);
    }

    fn sender(&self) -> Arc<Mutex<Option<mpsc::UnboundedSender<M>>>> {
        self.sender.clone()
    }

    fn message_queue(&self) -> Arc<Mutex<Vec<M>>> {
        self.message_queue.clone()
    }

    fn process_message_queue(&self) {
        let sender = self.sender();
        let mut sender = sender.lock().unwrap();
        if let Some(sender) = sender.as_mut() {
            let mut message_queue = self.message_queue.lock().unwrap();
            while let Some(message) = message_queue.pop() {
                if let Err(e) = sender.unbounded_send(message).map_err(|e| e.to_string()) {
                    tracing::error!("{}", e);
                    break;
                }
            }
        }
    }
}

// pub trait Connection {
//     fn new() -> Self;
//     fn with_channel(&mut self, sender: mpsc::UnboundedSender<M>);

//     fn sender(&self) -> Arc<Mutex<Option<mpsc::UnboundedSender<M>>>>;

//     fn send(&mut self, message: M) {
//         if let Ok(mut sender) = self.sender().lock() {
//             if let Some(sender) = sender.as_mut() {
//                 if let Err(e) = sender.unbounded_send(message).map_err(|e| e.to_string()) {
//                     tracing::error!("{}", e);
//                 }
//             } else {
//                 tracing::error!("Sender is not available");
//             }
//         } else {
//             tracing::error!("Failed to lock the sender");
//         }
//     }

//     fn try_send(&mut self, message: M) -> Result<(), Error> {
//         if let Ok(mut sender) = self.sender().lock() {
//             if let Some(sender) = sender.as_mut() {
//                 sender.unbounded_send(message);
//                 Ok(())
//             } else {
//                 Err(Error::TrySendError(format!("{:?}", message)))
//             }
//         } else {
//             Err(Error::TrySendError(format!("{:?}", message)))
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct BackEndConnection(Arc<Mutex<Option<mpsc::UnboundedSender<M>>>>);
// impl Connection for BackEndConnection {
//     fn new() -> Self {
//         Self(Arc::new(Mutex::new(None)))
//     }
//     fn with_channel(&mut self, sender: mpsc::UnboundedSender<M>) {
//         *self.0.lock().unwrap() = Some(sender);
//     }

//     fn sender(&self) -> Arc<Mutex<Option<mpsc::UnboundedSender<M>>>> {
//         self.0.clone()
//     }
// }
