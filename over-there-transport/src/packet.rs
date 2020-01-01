use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    /// ID used to collect packets forming a single message
    id: u32,

    /// Position within a collection of packets, starting at base 0
    index: u32,

    /// Indicates if this is the final packet in a message
    is_last: bool,
    // TODO: Add nonce; should we use an enum and combine with last?
    //       E.g. type:
    // nonce: Option<...>,

    // TODO: Add signature
    // signature: Option<...>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Packet {
    /// Represents metadata associated with the packet
    metadata: Metadata,

    #[serde(with = "serde_bytes")]
    /// Represents the actual data being transmitted
    data: Vec<u8>,
}

impl Packet {
    pub fn new(id: u32, index: u32, is_last: bool, data: Vec<u8>) -> Self {
        Packet {
            metadata: Metadata { id, index, is_last },
            data,
        }
    }

    /// Returns the size of metadata for packets
    pub fn metadata_size() -> usize {
        std::mem::size_of::<Metadata>()
    }

    /// Returns the id associated with the packet
    pub fn id(&self) -> u32 {
        self.metadata.id
    }

    /// Returns the index (position) of this packet relative to others with
    /// the same id
    pub fn index(&self) -> u32 {
        self.metadata.index
    }

    /// Returns whether or not this packet is the last in a multi-part collection
    pub fn is_last(&self) -> bool {
        self.metadata.is_last
    }

    /// Returns the bytes data held within the packet
    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    /// Serializes the packet to a collection of bytes
    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    /// Deserializes the slice of bytes to a single packet
    pub fn from_slice(slice: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(slice)
    }
}
