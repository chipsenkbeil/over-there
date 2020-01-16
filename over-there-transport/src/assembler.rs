use crate::packet::Packet;
use over_there_derive::Error;
use std::collections::HashMap;

#[derive(Debug, Error)]
pub enum AssemblerError {
    PacketExists { id: u32, index: u32 },
    PacketBeyondLastIndex { id: u32, index: u32 },
    PacketHasDifferentId { id: u32, expected_id: u32 },
    FinalPacketAlreadyExists { id: u32, index: u32 },
    IncompletePacketCollection,
}

struct PacketGroup {
    /// Collection of packets, where the key is the index of the packet
    packets: HashMap<u32, Packet>,

    /// The final index of the packet group, which we only know once we've
    /// received the final packet (can still be out of order)
    final_index: Option<u32>,
}

impl Default for PacketGroup {
    fn default() -> Self {
        Self {
            packets: HashMap::new(),
            final_index: None,
        }
    }
}

pub(crate) struct Assembler {
    /// Map of unique id to associated group of packets being assembled
    packet_groups: HashMap<u32, PacketGroup>,
}

impl Assembler {
    /// Adds a new packet to the assembler, consuming it for reconstruction
    pub fn add_packet(&mut self, packet: Packet) -> Result<(), AssemblerError> {
        let id = packet.id();
        let index = packet.index();
        let is_final = packet.is_final();

        // Check if we already have a group for this packet, otherwise create
        // a new group
        let group = self.packet_groups.entry(id).or_default();

        // Check if we already have this packet
        if group.packets.contains_key(&index) {
            return Err(AssemblerError::PacketExists { id, index });
        }

        // Check if we are adding a final packet when we already have one
        if let Some(last_index) = group.final_index {
            if is_final {
                return Err(AssemblerError::FinalPacketAlreadyExists {
                    id,
                    index: last_index,
                });
            }
        }

        // Check if we are trying to add a packet beyond the final one
        if group.final_index.map(|i| index > i).unwrap_or(false) {
            return Err(AssemblerError::PacketBeyondLastIndex { id, index });
        }

        // Add the packet to our group and, if it's final, mark it
        group.packets.insert(index, packet);
        if is_final {
            group.final_index = Some(index);
        }

        Ok(())
    }

    /// Removes the specified packet group from the assembler,
    /// returning the packets that were contained
    pub fn remove_group(&mut self, group_id: u32) -> Option<Vec<Packet>> {
        self.packet_groups
            .remove(&group_id)
            .as_mut()
            .map(|g| g.packets.drain().map(|e| e.1).collect())
    }

    /// Determines whether or not all packets have been added to the assembler
    pub fn verify(&self, group_id: u32) -> bool {
        self.packet_groups
            .get(&group_id)
            .and_then(|g| {
                let total_packets = g.packets.len() as u32;
                g.final_index.map(|i| i + 1 == total_packets)
            })
            .unwrap_or_default()
    }

    /// Reconstructs the data represented by the packets
    /// NOTE: This currently produces a copy of all data instead of passing
    ///       back out ownership
    pub fn assemble(&self, group_id: u32) -> Result<Vec<u8>, AssemblerError> {
        // Verify that we have all packets
        if !self.verify(group_id) {
            return Err(AssemblerError::IncompletePacketCollection);
        }

        // Grab the appropriate group, which we can now assume exists
        let group = self.packet_groups.get(&group_id).unwrap();

        // Gather references to packets in proper order
        let mut packets = group.packets.values().collect::<Vec<&Packet>>();
        packets.sort_unstable_by_key(|p| p.index());

        // Collect packet data into one unified binary representation
        // TODO: Improve by NOT cloning data
        Ok(packets.iter().flat_map(|p| p.data().clone()).collect())
    }
}

impl Default for Assembler {
    fn default() -> Self {
        Self {
            packet_groups: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{Metadata, PacketEncryption, PacketType};

    /// Make a packet with data; if last, mark as so with no nonce
    fn make_packet(id: u32, index: u32, is_last: bool, data: Vec<u8>) -> Packet {
        let r#type = if is_last {
            PacketType::Final {
                encryption: PacketEncryption::None,
            }
        } else {
            PacketType::NotFinal
        };
        let metadata = Metadata { id, index, r#type };
        Packet::new(metadata, Default::default(), data)
    }

    /// Make an empty packet; if last, mark as so with no nonce
    fn make_empty_packet(id: u32, index: u32, is_last: bool) -> Packet {
        make_packet(id, index, is_last, vec![])
    }

    #[test]
    fn add_packet_fails_if_packet_already_exists() {
        let mut a = Assembler::default();
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
            AssemblerError::PacketExists {
                id: eid,
                index: eindex,
            } => {
                assert_eq!(id, eid, "Unexpected index returned in error");
                assert_eq!(index, eindex, "Unexpected index returned in error");
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn add_packet_fails_if_adding_packet_beyond_last() {
        let mut a = Assembler::default();
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
            AssemblerError::PacketBeyondLastIndex {
                id: eid,
                index: eindex,
            } => {
                assert_eq!(id, eid, "Beyond packet id was different");
                assert_eq!(eindex, 1, "Beyond packet index was wrong");
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn add_packet_fails_if_packet_does_not_have_same_id() {
        let mut a = Assembler::default();
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
            AssemblerError::PacketHasDifferentId {
                id: actual_id,
                expected_id,
            } => {
                assert_eq!(actual_id, id + 1, "Actual id was different than provided");
                assert_eq!(expected_id, id, "Expected id was different from tracked");
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn add_packet_fails_if_last_packet_already_added() {
        let mut a = Assembler::default();

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
            AssemblerError::FinalPacketAlreadyExists { id, index } => {
                assert_eq!(id, 0, "Last packet id different than expected");
                assert_eq!(index, 1, "Last packet index different than expected");
            }
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn verify_yields_false_if_empty() {
        let a = Assembler::default();
        assert_eq!(a.verify(0), false);
    }

    #[test]
    fn verify_yields_false_if_missing_last_packet() {
        let mut a = Assembler::default();

        // Add first packet (index 0), still needing final packet
        let _ = a.add_packet(make_empty_packet(0, 0, false));

        assert_eq!(a.verify(0), false);
    }

    #[test]
    fn verify_yields_false_if_missing_first_packet() {
        let mut a = Assembler::default();

        // Add packet at end (index 1), still needing first packet
        assert_eq!(
            a.add_packet(make_empty_packet(0, 1, true)).is_ok(),
            true,
            "Unexpectedly failed to add a new packet",
        );

        assert_eq!(a.verify(0), false);
    }

    #[test]
    fn verify_yields_false_if_missing_inbetween_packet() {
        let mut a = Assembler::default();

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

        assert_eq!(a.verify(0), false);
    }

    #[test]
    fn verify_yields_true_if_have_all_packets() {
        let mut a = Assembler::default();

        assert_eq!(
            a.add_packet(make_empty_packet(0, 0, true)).is_ok(),
            true,
            "Unexpectedly failed to add a new packet",
        );

        assert_eq!(a.verify(0), true);
    }

    #[test]
    fn assemble_fails_if_not_verified() {
        let a = Assembler::default();

        let result = a.assemble(0);

        match result.unwrap_err() {
            AssemblerError::IncompletePacketCollection => (),
            e => panic!("Unexpected error {} received", e),
        }
    }

    #[test]
    fn assemble_yields_data_from_single_packet_if_complete() {
        let mut a = Assembler::default();
        let data: Vec<u8> = vec![1, 2, 3];

        // Try a single packet and collecting data
        let _ = a.add_packet(make_packet(0, 0, true, data.clone()));

        let collected_data = a.assemble(0).unwrap();
        assert_eq!(data, collected_data);
    }

    #[test]
    fn assemble_yields_combined_data_from_multiple_packets_if_complete() {
        let mut a = Assembler::default();
        let data: Vec<u8> = vec![1, 2, 3, 4, 5];

        // Try a multiple packets and collecting data
        let _ = a.add_packet(make_packet(0, 2, true, data[3..].to_vec()));
        let _ = a.add_packet(make_packet(0, 0, false, data[0..1].to_vec()));
        let _ = a.add_packet(make_packet(0, 1, false, data[1..3].to_vec()));

        let collected_data = a.assemble(0).unwrap();
        assert_eq!(data, collected_data);
    }
}
