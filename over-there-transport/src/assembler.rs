use crate::packet::Packet;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error {
    PacketExists(u32, u32),
    PacketBeyondLastIndex(u32, u32),
    PacketHasDifferentId(u32, u32),
    FinalPacketAlreadyExists(u32),
    IncompletePacketCollection,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::PacketExists(id, index) => write!(f, "Packet {}:{} already exists", id, index),
            Error::PacketBeyondLastIndex(id, index) => {
                write!(f, "Packet {}:{} beyond last index", id, index)
            }
            Error::PacketHasDifferentId(id, expected_id) => write!(
                f,
                "Packet has id {} whereas expected id {}",
                id, expected_id
            ),
            Error::FinalPacketAlreadyExists(final_packet_index) => write!(
                f,
                "Packet at index {} is already marked as the last packet",
                final_packet_index
            ),
            Error::IncompletePacketCollection => {
                write!(f, "Attempted to assemble without all packets!")
            }
        }
    }
}

pub(crate) struct Assembler {
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
    pub fn add_packet(&mut self, packet: Packet) -> Result<(), Error> {
        let index = packet.index();

        // Check if we already have this packet
        if self.packets.contains_key(&index) {
            return Err(Error::PacketExists(packet.id(), packet.index()));
        }

        // Check if we are adding a last packet when we already have one
        if let Some(last_index) = self.final_packet_index {
            if packet.is_last() {
                return Err(Error::FinalPacketAlreadyExists(last_index));
            }
        }

        // Check if we are trying to add a packet beyond the final one
        if self.final_packet_index.map(|i| index > i).unwrap_or(false) {
            return Err(Error::PacketBeyondLastIndex(packet.id(), packet.index()));
        }

        // Check if id does not match existing id
        if let Some(id) = self.id_for_packets {
            if packet.id() != id {
                return Err(Error::PacketHasDifferentId(packet.id(), id));
            }
        }

        // If it is our first time to add a packet, mark the id
        if self.packets.is_empty() {
            self.id_for_packets = Some(packet.id())
        }

        let pindex = packet.index();
        self.packets.insert(pindex, packet);

        // If we are adding the final packet, mark it
        if self.packets.get(&pindex).unwrap().is_last() {
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
    /// NOTE: This currently produces a copy of all data instead of passing
    ///       back out ownership
    pub fn assemble(&self) -> Result<Vec<u8>, Error> {
        // Verify that we have all packets
        if !self.verify() {
            return Err(Error::IncompletePacketCollection);
        }

        // Gather references to packets in proper order
        let mut packets = self.packets.values().collect::<Vec<&Packet>>();
        packets.sort_unstable_by_key(|p| p.index());

        // Collect packet data into one unified binary representation
        // TODO: Improve by NOT cloning data
        let data: Vec<u8> = packets.iter().flat_map(|p| p.data().clone()).collect();

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_empty_packet(id: u32, index: u32, is_last: bool) -> Packet {
        Packet::new(id, index, is_last, vec![])
    }

    #[test]
    fn add_packet_fails_if_packet_already_exists() {
        let mut a = Assembler::new();
        let id = 123;
        let index = 999;

        // Add first packet successfully
        let result = a.add_packet(make_empty_packet(id, index, false));
        assert_eq!(
            result.is_ok(),
            true,
            "Expected success for adding first packet, but got {}",
            result.unwrap_err(),
        );

        // Fail if adding packet with same index
        match a
            .add_packet(make_empty_packet(id, index, false))
            .unwrap_err()
        {
            Error::PacketExists(eid, eindex) => {
                assert_eq!(id, eid, "Unexpected index returned in error");
                assert_eq!(index, eindex, "Unexpected index returned in error");
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn add_packet_fails_if_adding_packet_beyond_last() {
        let mut a = Assembler::new();
        let id = 123;

        // Add first packet successfully
        let result = a.add_packet(make_empty_packet(id, 0, true));
        assert_eq!(
            result.is_ok(),
            true,
            "Expected success for adding first packet, but got {}",
            result.unwrap_err(),
        );

        // Fail if adding packet after final packet
        match a.add_packet(make_empty_packet(id, 1, false)).unwrap_err() {
            Error::PacketBeyondLastIndex(eid, eindex) => {
                assert_eq!(id, eid, "Beyond packet id was different");
                assert_eq!(eindex, 1, "Beyond packet index was wrong");
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn add_packet_fails_if_packet_does_not_have_same_id() {
        let mut a = Assembler::new();
        let id = 999;

        // Add first packet successfully
        let result = a.add_packet(make_empty_packet(id, 0, false));
        assert_eq!(
            result.is_ok(),
            true,
            "Expected success for adding first packet, but got {}",
            result.unwrap_err(),
        );

        // Fail if adding packet after final packet
        match a
            .add_packet(make_empty_packet(id + 1, 1, false))
            .unwrap_err()
        {
            Error::PacketHasDifferentId(actual_id, expected_id) => {
                assert_eq!(actual_id, id + 1, "Actual id was different than provided");
                assert_eq!(expected_id, id, "Expected id was different from tracked");
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn add_packet_fails_if_last_packet_already_added() {
        let mut a = Assembler::new();

        // Make the second packet (index) be the last packet
        let result = a.add_packet(make_empty_packet(0, 1, true));
        assert_eq!(
            result.is_ok(),
            true,
            "Expected success for adding first packet, but got {}",
            result.unwrap_err(),
        );

        // Fail if making the first packet (index) be the last packet
        // when we already have a last packet
        match a.add_packet(make_empty_packet(0, 0, true)).unwrap_err() {
            Error::FinalPacketAlreadyExists(actual_last_index) => {
                assert_eq!(
                    actual_last_index, 1,
                    "Last packet index different than expected"
                );
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn verify_yields_false_if_empty() {
        let a = Assembler::new();
        assert_eq!(a.verify(), false);
    }

    #[test]
    fn verify_yields_false_if_missing_last_packet() {
        let mut a = Assembler::new();

        // Add first packet (index 0), still needing final packet
        let _ = a.add_packet(make_empty_packet(0, 0, false));

        assert_eq!(a.verify(), false);
    }

    #[test]
    fn verify_yields_false_if_missing_first_packet() {
        let mut a = Assembler::new();

        // Add packet at end (index 1), still needing first packet
        assert_eq!(
            a.add_packet(make_empty_packet(0, 1, true)).is_ok(),
            true,
            "Unexpectedly failed to add a new packet",
        );

        assert_eq!(a.verify(), false);
    }

    #[test]
    fn verify_yields_false_if_missing_inbetween_packet() {
        let mut a = Assembler::new();

        // Add packet at beginning (index 0)
        assert_eq!(
            a.add_packet(make_empty_packet(0, 0, false)).is_ok(),
            true,
            "Unexpectedly failed to add a new packet",
        );

        // Add packet at end (index 2)
        assert_eq!(
            a.add_packet(make_empty_packet(0, 2, true)).is_ok(),
            true,
            "Unexpectedly failed to add a new packet",
        );

        assert_eq!(a.verify(), false);
    }

    #[test]
    fn verify_yields_true_if_have_all_packets() {
        let mut a = Assembler::new();

        assert_eq!(
            a.add_packet(make_empty_packet(0, 0, true)).is_ok(),
            true,
            "Unexpectedly failed to add a new packet",
        );

        assert_eq!(a.verify(), true);
    }

    #[test]
    fn assemble_fails_if_not_verified() {
        let a = Assembler::new();

        let result = a.assemble();

        match result.unwrap_err() {
            Error::IncompletePacketCollection => (),
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn assemble_yields_data_from_single_packet_if_complete() {
        let mut a = Assembler::new();
        let data: Vec<u8> = vec![1, 2, 3];

        // Try a single packet and collecting data
        let _ = a.add_packet(Packet::new(0, 0, true, data.clone()));

        let collected_data = a.assemble().unwrap();
        assert_eq!(data, collected_data);
    }

    #[test]
    fn assemble_yields_combined_data_from_multiple_packets_if_complete() {
        let mut a = Assembler::new();
        let data: Vec<u8> = vec![1, 2, 3, 4, 5];

        // Try a multiple packets and collecting data
        let _ = a.add_packet(Packet::new(0, 2, true, data[3..].to_vec()));
        let _ = a.add_packet(Packet::new(0, 0, false, data[0..1].to_vec()));
        let _ = a.add_packet(Packet::new(0, 1, false, data[1..3].to_vec()));

        let collected_data = a.assemble().unwrap();
        assert_eq!(data, collected_data);
    }
}