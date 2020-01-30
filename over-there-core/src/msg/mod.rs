pub mod content;

use chrono::prelude::{DateTime, Utc};
use content::Content;
use over_there_derive::Error;
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum MsgError {
    EncodeMsg(rmp_serde::encode::Error),
    DecodeMsg(rmp_serde::decode::Error),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// ID associated with a request or response
    pub id: u32,

    /// The time at which the message was created
    pub creation_date: DateTime<Utc>,
}

impl Default for Header {
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
    pub fn to_vec(&self) -> Result<Vec<u8>, MsgError> {
        rmp_serde::to_vec(&self).map_err(MsgError::EncodeMsg)
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, MsgError> {
        rmp_serde::from_read_ref(slice).map_err(MsgError::DecodeMsg)
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

/// Produce a new message from the content with existing message
/// being the parent
impl From<(Content, Msg)> for Msg {
    fn from((content, msg): (Content, Msg)) -> Self {
        Self::from((content, msg.header))
    }
}

/// Produce a new message from the content with existing header
/// being the parent
impl From<(Content, Header)> for Msg {
    fn from((content, header): (Content, Header)) -> Self {
        let mut new_msg = Self::from(content);
        new_msg.parent_header = Some(header);
        new_msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_should_produce_a_new_msg_with_that_content() {
        let msg = Msg::from(Content::Heartbeat);

        // Verify creation date was set to around now
        assert!(
            Utc::now()
                .signed_duration_since(msg.header.creation_date)
                .num_milliseconds()
                >= 0,
            "Unexpected creation date: {:?}",
            msg.header.creation_date
        );

        // Verify that our message was set to the right type
        match msg.content {
            Content::Heartbeat => (),
            x => panic!("Unexpected content: {:?}", x),
        }
    }
}
