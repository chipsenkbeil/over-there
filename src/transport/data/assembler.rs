use super::Packet;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug)]
pub enum AssemblerError {
    PacketExists(u32),
    PacketBeyondLastIndex(u32, u32),
    PacketHasDifferentId(u32, u32),
    IncompletePacketCollection,
}

impl Error for AssemblerError {}

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            AssemblerError::PacketExists(index) => write!(f, "Packet {} already exists", index),
            AssemblerError::PacketBeyondLastIndex(index, last_index) => {
                write!(f, "Packet {} beyond last index of {}", index, last_index)
            }
            AssemblerError::PacketHasDifferentId(id, expected_id) => write!(
                f,
                "Packet has id {} whereas expected id {}",
                id, expected_id
            ),
            AssemblerError::IncompletePacketCollection => {
                write!(f, "Attempted to assemble without all packets!")
            }
        }
    }
}

pub struct Assembler {
    packets: HashMap<u32, Packet>,
    final_packet_index: Option<u32>,
    id_for_packets: Option<u32>,
}

impl Assembler {
    /// Creates a new, empty instance
    pub fn new() -> Self {
        Assembler {
            packets: HashMap::new(),
            final_packet_index: None,
            id_for_packets: None,
        }
    }

    /// Adds a new packet to the assembler, consuming it for reconstruction
    pub fn add_packet(&mut self, packet: Packet) -> Result<(), AssemblerError> {
        // Check if we already have this packet
        if self.packets.contains_key(&packet.metadata.index) {
            return Err(AssemblerError::PacketExists(packet.metadata.index));
        }

        // Check if we are trying to add a packet beyond the final one
        let index = packet.metadata.index;
        if self.final_packet_index.map(|i| index > i).unwrap() {
            return Err(AssemblerError::PacketBeyondLastIndex(
                index,
                self.final_packet_index.unwrap(),
            ));
        }

        // If it is our first time to add a packet, mark the id
        if self.packets.is_empty() {
            self.id_for_packets = Some(packet.metadata.id)
        }

        let pindex = packet.metadata.index;
        self.packets.insert(pindex, packet);

        // If we are adding the final packet, mark it
        if self.packets.get(&pindex).unwrap().metadata.is_last {
            self.final_packet_index = Some(pindex);
        }

        Ok(())
    }

    /// Determines whether or not all packets have been added to the assembler
    pub fn verify(&self) -> bool {
        let total_packets = self.packets.len() as u32;
        self.final_packet_index
            .map(|i| i + 1 == total_packets)
            .unwrap_or(false)
    }

    /// Reconstructs the data represented by the packets
    pub fn assemble(&self) -> Result<Vec<&u8>, AssemblerError> {
        // Verify that we have all packets
        if !self.verify() {
            return Err(AssemblerError::IncompletePacketCollection);
        }

        // Gather references to packets in proper order
        let mut packets = self.packets.values().collect::<Vec<&Packet>>();
        packets.sort_unstable_by_key(|p| p.metadata.index);

        // Collect packet data into one unified binary representation
        // TODO: Fix using clone on packet data
        let data: Vec<&u8> = packets.iter().flat_map(|p| &p.data).collect();

        Ok(data)
    }
}
