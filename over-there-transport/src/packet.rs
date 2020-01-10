use over_there_auth::Digest;
use over_there_crypto::Nonce;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum PacketType {
    /// Represents packets that are not the final in a collection
    NotFinal,

    /// Represents final packet and includes nonce if used to encrypt data
    Final { nonce: Option<Nonce> },
}

impl PacketType {
    pub fn nonce(&self) -> Option<&Nonce> {
        match self {
            Self::NotFinal => None,
            Self::Final { nonce } => nonce.as_ref(),
        }
    }

    pub fn is_final(&self) -> bool {
        match self {
            Self::NotFinal => false,
            Self::Final { nonce: _ } => true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Metadata {
    /// ID used to collect packets forming a single message
    pub(crate) id: u32,

    /// Position within a collection of packets, starting at base 0
    pub(crate) index: u32,

    /// Type of packet, indicating if it is the final packet and any
    /// extra data associated with the final packet
    pub(crate) r#type: PacketType,
}

impl Metadata {
    /// Serializes the metadata to a collection of bytes
    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Packet {
    /// Represents metadata associated with the packet
    metadata: Metadata,

    /// The signature used to validate the data contained in the packet
    signature: Digest,

    #[serde(with = "serde_bytes")]
    /// Represents the actual data being transmitted
    data: Vec<u8>,
}

impl Packet {
    pub fn new(metadata: Metadata, signature: Digest, data: Vec<u8>) -> Self {
        Packet {
            metadata,
            signature,
            data,
        }
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
    pub fn is_final(&self) -> bool {
        self.metadata.r#type.is_final()
    }

    /// Returns the signature associated with the packet's data
    pub fn signature(&self) -> &Digest {
        &self.signature
    }

    /// Creates content used when producing and verifying a signature
    pub(crate) fn content_for_signature(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        Ok([self.metadata.to_vec()?, self.data.to_vec()].concat())
    }

    /// Returns the nonce contained in the packet, if it has one
    pub fn nonce(&self) -> Option<&Nonce> {
        self.metadata.r#type.nonce()
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
