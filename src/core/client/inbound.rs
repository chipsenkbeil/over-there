use crate::Msg;
use std::net::SocketAddr;
use tokio::sync::mpsc::Receiver;

pub struct InboundMsgReader<T> {
    rx: Receiver<(Msg, SocketAddr, T)>,
}

impl<T> InboundMsgReader<T> {
    pub fn new(rx: Receiver<(Msg, SocketAddr, T)>) -> Self {
        Self { rx }
    }

    pub async fn next(&mut self) -> Option<Msg> {
        match self.rx.recv().await {
            Some((msg, _, _)) => Some(msg),
            _ => None,
        }
    }
}
