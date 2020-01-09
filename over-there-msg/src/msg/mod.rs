pub mod content;

use chrono::prelude::{DateTime, Utc};
use content::Content;
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Msg {
    /// ID associated with a request or response
    pub id: u32,

    /// The time at which the message was created
    pub creation_date: DateTime<Utc>,

    /// Content within the message
    pub content: Content,
}

impl Msg {
    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(slice)
    }
}

impl From<Content> for Msg {
    fn from(content: Content) -> Self {
        Self {
            id: random(),
            creation_date: Utc::now(),
            content,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_should_produce_a_new_msg_with_that_content() {
        let msg = Msg::from(Content::HeartbeatRequest);

        // Verify creation date was set to around now
        assert!(
            Utc::now()
                .signed_duration_since(msg.creation_date)
                .num_milliseconds()
                >= 0,
            "Unexpected creation date: {:?}",
            msg.creation_date
        );

        // Verify that our message was set to the right type
        match msg.content {
            Content::HeartbeatRequest => (),
            x => panic!("Unexpected content: {:?}", x),
        }
    }
}
