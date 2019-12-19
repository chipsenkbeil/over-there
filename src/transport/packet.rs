use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use std::mem;

#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    group_id: u32,
    index: u32,
    total_entries: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {
    metadata: Option<Metadata>,

    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

impl Packet {
    const METADATA_SIZE: u32 = mem::size_of::<Metadata>() as u32;

    pub fn data_to_multipart(data: Vec<u8>, max_data_per_packet: u32) -> Vec<Self> {
        // TODO: How can we get the overhead size
        if data.len() as u32 <= max_data_per_packet {
            vec![Packet {
                metadata: None,
                data,
            }]
        } else {
            // TODO: Create unique group id
            let group_id = 0;
            let chunk_size = max_data_per_packet - Self::METADATA_SIZE;
            let chunks = data.chunks(chunk_size as usize);
            let total_chunks = chunks.len();
            let packets = chunks
                .enumerate()
                .map(|(index, chunk)| Packet {
                    metadata: Some(Metadata {
                        group_id,
                        index: index as u32,
                        total_entries: total_chunks as u32,
                    }),
                    data: chunk.to_vec(),
                })
                .collect::<Vec<Packet>>();
            packets
        }
    }

    pub fn multipart_to_data(mut packets: Vec<Packet>) -> Result<Vec<u8>, Error> {
        // Check the integrity to ensure that we have all pieces
        // TODO: Also check we have each packet by index (count through and
        //       fail if we find that we skipped an index)
        if packets.iter().any(|p| p.metadata.is_none()) {
            let e = Error::new(ErrorKind::Other, "");
            Err(e)
        } else {
            // Join packet data in order
            packets.sort_unstable_by_key(|p| p.metadata.as_ref().unwrap().index);
            // TODO: Fix using clone on packet data
            let data: Vec<u8> = packets.iter().flat_map(|p| p.data.clone()).collect();
            Ok(data)
        }
    }

    pub fn is_multipart(&self) -> bool {
        self.metadata.is_some()
    }

    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    pub fn from_vec(v: &Vec<u8>) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(v)
    }
}
