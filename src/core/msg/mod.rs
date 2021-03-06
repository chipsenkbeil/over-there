pub mod content;

use chrono::prelude::{DateTime, Utc};
use content::{Content, Reply, Request};
use derive_more::{Display, Error};
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Debug, Display, Error)]
pub enum MsgError {
    AssembleMsg(serde_cbor::Error),
    DisassembleMsg(serde_cbor::Error),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// ID associated with a request or reply
    pub id: u32,

    /// The time at which the message was created
    pub creation_date: DateTime<Utc>,
}

impl Header {
    /// Creates a new Header with the provided ID
    pub fn with_id(id: u32) -> Self {
        let mut header = Header::default();
        header.id = id;
        header
    }
}

impl Default for Header {
    /// Creates a new Header with a random ID
    fn default() -> Self {
        Self {
            id: random(),
            creation_date: Utc::now(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Msg {
    /// Information associated with this message
    pub header: Header,

    /// Information associated with the parent of this message
    /// to provide origin context
    pub parent_header: Option<Header>,

    /// Content within the message
    pub content: Content,
}

impl Msg {
    pub fn new(content: Content, parent_header: Option<Header>) -> Self {
        Self {
            header: Header::default(),
            parent_header,
            content,
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, MsgError> {
        // NOTE: Cannot use to_vec_packed here as it fails to deserialize
        serde_cbor::ser::to_vec(&self).map_err(MsgError::AssembleMsg)
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, MsgError> {
        serde_cbor::from_slice(slice).map_err(MsgError::DisassembleMsg)
    }

    /// Sets the parent header of this msg with that of the provided header
    pub fn with_parent_header(&mut self, header: Header) -> &mut Self {
        self.parent_header = Some(header);
        self
    }

    /// Sets the parent header of this msg with that of the provided parent
    pub fn with_parent(&mut self, parent: &Self) -> &mut Self {
        self.with_parent_header(parent.header.clone())
    }
}

/// Produce a new message from the content with no parent
impl From<Content> for Msg {
    fn from(content: Content) -> Self {
        Self {
            header: Header::default(),
            parent_header: None,
            content,
        }
    }
}

impl From<Request> for Msg {
    fn from(request: Request) -> Self {
        Self::from(Content::from(request))
    }
}

impl From<Reply> for Msg {
    fn from(reply: Reply) -> Self {
        Self::from(Content::from(reply))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_header_should_have_creation_date_set_to_now() {
        let header = Header::default();

        // Verify creation date was set to around now
        assert!(
            Utc::now()
                .signed_duration_since(header.creation_date)
                .num_milliseconds()
                >= 0,
            "Unexpected creation date: {:?}",
            header.creation_date
        );
    }

    #[test]
    fn from_content_should_create_msg_with_content() {
        let msg = Msg::from(Request::Heartbeat);

        assert_eq!(msg.parent_header, None);

        match msg.content {
            Content::Request(Request::Heartbeat) => (),
            x => panic!("Unexpected content: {:?}", x),
        }
    }

    #[test]
    fn with_parent_header_should_set_header() {
        let mut msg = Msg::from(Reply::Heartbeat);
        let header = Header::default();

        msg.with_parent_header(header.clone());

        assert_eq!(msg.parent_header, Some(header));
    }

    #[test]
    fn with_parent_should_set_header_to_that_of_parent() {
        let mut msg = Msg::from(Reply::Heartbeat);
        let parent = Msg::from(Request::Heartbeat);

        msg.with_parent(&parent);

        assert_eq!(msg.parent_header, Some(parent.header));
    }
}
