pub mod assembler;
pub mod disassembler;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    /// ID used to collect packets forming a single message
    id: u32,

    /// Position within a collection of packets, starting at base 0
    index: u32,

    /// Indicates if this is the final packet in a message
    is_last: bool,
}

impl Metadata {
    fn new(id: u32, index: u32, is_last: bool) -> Self {
        Metadata { id, index, is_last }
    }

    fn size() -> u32 {
        std::mem::size_of::<Self>() as u32
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {
    /// Represents metadata associated with the packet
    metadata: Metadata,

    #[serde(with = "serde_bytes")]
    /// Represents the actual data being transmitted
    data: Vec<u8>,
}

impl Packet {
    /// Returns the size of metadata for packets
    pub fn metadata_size() -> u32 {
        Metadata::size()
    }

    /// Indicates whether or not this packet is part of a series of packets
    /// representing one collection of data
    pub fn is_multipart(&self) -> bool {
        self.metadata.index > 0 || !self.metadata.is_last
    }

    /// Returns the id associated with the packet
    pub fn get_id(&self) -> u32 {
        self.metadata.id
    }

    /// Returns the index (position) of this packet relative to others with
    /// the same id
    pub fn get_index(&self) -> u32 {
        self.metadata.index
    }

    /// Returns whether or not this packet is the last in a multi-part collection
    pub fn is_last(&self) -> bool {
        self.metadata.is_last
    }

    /// Returns the bytes data held within the packet
    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }

    /// Serializes the packet to a collection of bytes
    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    /// Deserializes the collection of bytes to a single packet
    pub fn from_vec(v: &Vec<u8>) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(v)
    }

    /// Deserializes the slice of bytes to a single packet
    pub fn from_slice(slice: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod is_multipart {
        use super::*;

        #[test]
        fn yields_false_if_first_and_only() {
            let result =
                disassembler::make_packets_from_data(0, vec![1, 2, 3], Metadata::size() + 3);
            let p = &result.unwrap()[0];

            assert_eq!(p.is_multipart(), false);
        }

        #[test]
        fn yields_true_if_first_of_many() {
            let result =
                disassembler::make_packets_from_data(0, vec![1, 2, 3], Metadata::size() + 1);
            let p = &result.unwrap()[0];

            assert_eq!(p.is_multipart(), true);
        }

        #[test]
        fn yields_true_if_one_of_many() {
            let result =
                disassembler::make_packets_from_data(0, vec![1, 2, 3], Metadata::size() + 1);
            let p = &result.unwrap()[1];

            assert_eq!(p.is_multipart(), true);
        }

        #[test]
        fn yields_true_if_last_of_many() {
            let result =
                disassembler::make_packets_from_data(0, vec![1, 2, 3], Metadata::size() + 1);
            let p = &result.unwrap()[2];

            assert_eq!(p.is_multipart(), true);
        }
    }
}
