mod heartbeat;
mod unknown;
mod version;

use crate::{
    msg::{
        content::{Content, ContentType},
        Msg, MsgError,
    },
    state::State,
};
use over_there_derive::Error;
use over_there_transport::{NetSend, NetSendError};

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
    NetSendError(NetSendError),
    UnexpectedContent,
    Unknown,
}

/// Looks up an appropriate function pointer for the given content type
pub fn route<NS: NetSend>(
    content_type: ContentType,
) -> fn(&mut State, Msg, &NS) -> Result<(), ActionError> {
    match content_type {
        ContentType::HeartbeatRequest => heartbeat::heartbeat_request,
        ContentType::HeartbeatResponse => heartbeat::heartbeat_response,

        ContentType::VersionRequest => version::version_request,
        ContentType::VersionResponse => version::version_response,

        // TODO: Remove unknown by completing all other content types
        _ => unknown::unknown,
    }
}

/// Evaluate a message's content and potentially respond using the provided
/// netsend component
pub fn execute<NS: NetSend>(state: &mut State, msg: Msg, ns: &NS) -> Result<(), ActionError> {
    (route(ContentType::from(&msg.content)))(state, msg, ns)
}

/// Sends a response to the originator of a msg
pub(crate) fn respond<NS: NetSend>(
    ns: &NS,
    content: Content,
    parent_msg: Msg,
) -> Result<(), ActionError> {
    let new_msg = Msg::from((content, parent_msg));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;
    ns.send(&data).map_err(ActionError::NetSendError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[derive(Clone)]
    pub struct MockNetSend {
        last_sent: RefCell<Option<Vec<u8>>>,
        addr: SocketAddr,
    }

    impl MockNetSend {
        pub fn take_last_sent(&mut self) -> Option<Vec<u8>> {
            self.last_sent.borrow_mut().take()
        }
    }

    impl Default for MockNetSend {
        fn default() -> Self {
            Self {
                last_sent: RefCell::new(None),
                addr: SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), 0),
            }
        }
    }

    impl NetSend for MockNetSend {
        fn send(&self, data: &[u8]) -> Result<(), NetSendError> {
            *self.last_sent.borrow_mut() = Some(data.to_vec());
            Ok(())
        }

        fn addr(&self) -> SocketAddr {
            self.addr
        }
    }
}
