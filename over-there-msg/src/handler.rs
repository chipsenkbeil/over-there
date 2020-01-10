use crate::msg::transmitter::MsgTransmitterError;
use crate::msg::{content::ContentType, Msg};
use over_there_derive::Error;
use rand::random;
use std::collections::HashMap;
use std::rc::Rc;

pub type HandlerId = u32;
pub type HandlerMetadata = HashMap<&'static str, String>;
pub type HandlerInternalStore = HashMap<HandlerId, HandlerContainer>;
pub type HandlerFp = fn(&HandlerContext) -> Result<(), Box<dyn std::error::Error>>;
pub type PredicateFp = fn(&HandlerContext) -> bool;

#[derive(Debug, Error)]
pub enum HandlerError {
    StoreNotAvailable,
    StoreIdCollision,
}

pub trait HandlerSender {
    fn send(&self, msg: Msg) -> Result<(), MsgTransmitterError>;
}

pub struct HandlerContext<'a, 'b, 'c, 'd> {
    /// ID associated with the handler that can be used to remove it
    id: &'a HandlerId,

    /// Reference to the store containing the handler
    store: Rc<HandlerInternalStore>,

    /// Represents the incoming msg
    pub msg: &'b Msg,

    /// Used to send a msg back to the originator of the incoming msg
    sender: &'d dyn HandlerSender,

    /// Provides additional metadata used for special situations
    pub metadata: &'c HandlerMetadata,
}

impl<'a, 'b, 'c, 'd> HandlerContext<'a, 'b, 'c, 'd> {
    /// Sends a new msg to the originator of the incoming msg
    pub fn send(&self, msg: Msg) -> Result<(), MsgTransmitterError> {
        self.sender.send(msg)
    }

    /// Marks the handler as being complete and will de-register it from
    /// the associated store
    pub fn done(&mut self) -> bool {
        if let Some(s) = Rc::get_mut(&mut self.store) {
            s.remove(&self.id);
            true
        } else {
            false
        }
    }
}

pub struct HandlerContainer {
    id: HandlerId,

    /// Callback to be invoked on specific msgs
    handler: HandlerFp,

    /// Used to determine if a handler should be invoked for a msg
    predicate: PredicateFp,

    /// Additional information associated with the handler
    metadata: HandlerMetadata,
}

pub struct HandlerStore {
    store: Rc<HandlerInternalStore>,
}

impl HandlerStore {
    pub fn new() -> Self {
        Self {
            store: Rc::new(HashMap::new()),
        }
    }

    /// Register a handler to be invoked when a msg is received whose
    /// content type matches the specified type
    pub fn register_for_content_type(
        &mut self,
        content_type: ContentType,
        handler: HandlerFp,
    ) -> Result<HandlerId, HandlerError> {
        let mut metadata: HandlerMetadata = HashMap::new();
        metadata.insert("content_type", content_type.to_string());

        self.register_with_predicate(
            handler,
            |ctx| {
                use std::str::FromStr;
                let content_type =
                    ContentType::from_str(ctx.metadata.get("content_type").unwrap().as_str())
                        .unwrap();
                ContentType::from(&ctx.msg.content) == content_type
            },
            metadata,
        )
    }

    /// Register a handler to be invoked when a msg is received whose
    /// origin comes from a msg with the specified id
    pub fn register_for_origin(
        &mut self,
        id: u32,
        handler: HandlerFp,
    ) -> Result<HandlerId, HandlerError> {
        let mut metadata: HandlerMetadata = HashMap::new();
        metadata.insert("id", id.to_string());

        self.register_with_predicate(
            handler,
            |ctx| {
                let id: u32 = ctx.metadata.get("id").unwrap().parse().unwrap();
                ctx.msg
                    .parent_header
                    .as_ref()
                    .map(|h| h.id == id)
                    .unwrap_or_default()
            },
            metadata,
        )
    }

    /// Register a handler to be invoked whenever the predicate yields true,
    /// and provide additional metadata that can be provided to both the
    /// handler and predicate
    pub fn register_with_predicate(
        &mut self,
        handler: HandlerFp,
        predicate: PredicateFp,
        metadata: HandlerMetadata,
    ) -> Result<HandlerId, HandlerError> {
        let id: u32 = random();
        self.register(id, handler, predicate, metadata)
    }

    fn register(
        &mut self,
        id: HandlerId,
        handler: HandlerFp,
        predicate: PredicateFp,
        metadata: HandlerMetadata,
    ) -> Result<HandlerId, HandlerError> {
        let handler_container = HandlerContainer {
            id,
            handler,
            predicate,
            metadata,
        };

        Rc::get_mut(&mut self.store)
            .ok_or(HandlerError::StoreNotAvailable)
            .and_then(|s| {
                if s.contains_key(&id) {
                    Err(HandlerError::StoreIdCollision)
                } else {
                    s.insert(id, handler_container);
                    Ok(id)
                }
            })
    }

    pub fn call(
        &mut self,
        msg: &Msg,
        sender: &dyn HandlerSender,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for con in self.store.values() {
            let ctx = HandlerContext {
                id: &con.id,
                store: Rc::clone(&self.store),
                msg,
                sender,
                metadata: &con.metadata,
            };

            if (con.predicate)(&ctx) {
                (con.handler)(&ctx)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Content;

    struct FakeSender {}

    impl HandlerSender for FakeSender {
        fn send(&self, _msg: Msg) -> Result<(), MsgTransmitterError> {
            Ok(())
        }
    }

    #[test]
    fn can_invoke() {
        let mut store = HandlerStore::new();
        assert!(store
            .register_for_content_type(ContentType::HeartbeatRequest, |_ctx| {
                println!("TEST");
                Ok(())
            })
            .is_ok());
        let sender = FakeSender {};
        assert!(
            store
                .call(&Msg::from(Content::HeartbeatRequest), &sender)
                .is_ok(),
            "Call failed"
        );
    }
}
