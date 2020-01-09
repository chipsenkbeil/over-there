use crate::msg::{content::ContentType, Msg};
use crate::transmitter::MsgTransmitterError;
use std::collections::HashMap;

pub type SendMsg = fn(Msg) -> Result<(), MsgTransmitterError>;
pub type Handler = fn(&Msg, SendMsg) -> Result<(), Box<dyn std::error::Error>>;

pub struct HandlerStore {
    store: HashMap<ContentType, Vec<Handler>>,
}

impl HandlerStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn register(&mut self, content_type: ContentType, handler: Handler) {
        self.store
            .entry(content_type)
            .or_insert(Vec::new())
            .push(handler);
    }

    pub fn call(&self, msg: Msg, send_msg: SendMsg) {
        let content_type = ContentType::from(&msg.content);
        self.store
            .get(&content_type)
            .map(|handlers| handlers.iter().map(|h| h(&msg, send_msg)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_invoke() {
        let mut store = HandlerStore::new();
        store.register(ContentType::HeartbeatRequest, |msg, send| Ok(()));
        panic!("GOT HERE");
    }
}
