use crate::msg::{content::ContentType, Msg};
use crate::transmitter::MsgTransmitterError;
use over_there_derive::Error;
use rand::random;
use std::collections::HashMap;
use std::rc::Rc;

pub type HandlerId = u32;
pub type HandlerMetadata = HashMap<&'static str, String>;
pub type HandlerInternalStore = HashMap<HandlerId, HandlerContainer>;
pub type SendMsgFp = fn(Msg) -> Result<(), MsgTransmitterError>;
pub type HandlerFp =
    fn(&HandlerContext, &HandlerMetadata) -> Result<(), Box<dyn std::error::Error>>;
pub type PredicateFp = fn(&HandlerContext, &HandlerMetadata) -> bool;

#[derive(Debug, Error)]
pub enum HandlerError {
    StoreNotAvailable,
}

pub struct HandlerContext<'a, 'b> {
    /// ID associated with the handler that can be used to remove it
    id: &'a HandlerId,

    /// Reference to the store containing the handler
    store: Rc<HandlerInternalStore>,

    /// Represents the incoming msg
    pub msg: &'b Msg,

    /// Used to send a msg back to the originator of the incoming msg
    send_msg: SendMsgFp,
}

impl<'a, 'b> HandlerContext<'a, 'b> {
    /// Sends a new msg to the originator of the incoming msg
    pub fn send(&self, msg: Msg) -> Result<(), MsgTransmitterError> {
        (self.send_msg)(msg)
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

        self.register(
            handler,
            |ctx, m| {
                use std::str::FromStr;
                let content_type =
                    ContentType::from_str(m.get("content_type").unwrap().as_str()).unwrap();
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

        self.register(
            handler,
            |ctx, m| {
                let id: u32 = m.get("id").unwrap().parse().unwrap();
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
    pub fn register(
        &mut self,
        handler: HandlerFp,
        predicate: PredicateFp,
        metadata: HandlerMetadata,
    ) -> Result<HandlerId, HandlerError> {
        let id: u32 = random();
        // 1. Move done() to impl on context
        //
        // 2. Update call to not take full context so we can construct it
        //
        // !! Will still need to pass msg (as ref) and send func
        //
        // -- Could make send func a trait implementing send for a msg and
        //    make the call method a generic?
        //
        // Do we need to have a function pointer for predicate? Could we just
        // have two separate predicate types for origin and content type?
        //
        // Do we need a metadata hashmap? Or can the ctx store this in some
        // other way?
        let handler_container = HandlerContainer {
            id,
            handler,
            predicate,
            metadata,
        };

        // TODO: Handle collision; for now, we can safely assume that a small
        //       number of handlers won't have a collision
        Rc::get_mut(&mut self.store)
            .ok_or(HandlerError::StoreNotAvailable)
            .map(|s| {
                s.insert(id, handler_container);
                id
            })
    }

    pub fn call(
        &mut self,
        msg: &Msg,
        send_msg: SendMsgFp,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for con in self.store.values() {
            let ctx = HandlerContext {
                id: &con.id,
                store: Rc::clone(&self.store),
                msg,
                send_msg,
            };

            if (con.predicate)(&ctx, &con.metadata) {
                (con.handler)(&ctx, &con.metadata)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_invoke() {
        let mut store = HandlerStore::new();
        assert!(store
            .register_for_content_type(ContentType::HeartbeatRequest, |_ctx, _metadata| {
                println!("TEST");
                Ok(())
            })
            .is_ok());
        panic!("TODO: Implement real tests");
    }
}
