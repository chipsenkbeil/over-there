use super::errors::AssemblerError;
use super::Packet;
use std::collections::HashMap;

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
        let index = packet.metadata.index;

        // Check if we already have this packet
        if self.packets.contains_key(&index) {
            return Err(AssemblerError::PacketExists(packet.metadata.index));
        }

        // Check if we are adding a last packet when we already have one
        if let Some(last_index) = self.final_packet_index {
            if packet.is_last() {
                return Err(AssemblerError::FinalPacketAlreadyExists(last_index));
            }
        }

        // Check if we are trying to add a packet beyond the final one
        if self.final_packet_index.map(|i| index > i).unwrap_or(false) {
            return Err(AssemblerError::PacketBeyondLastIndex(
                index,
                self.final_packet_index.unwrap(),
            ));
        }

        // Check if id does not match existing id
        if let Some(id) = self.id_for_packets {
            if packet.metadata.id != id {
                return Err(AssemblerError::PacketHasDifferentId(packet.metadata.id, id));
            }
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
    /// NOTE: This currently produces a copy of all data instead of passing
    ///       back out ownership
    pub fn assemble(&self) -> Result<Vec<u8>, AssemblerError> {
        // Verify that we have all packets
        if !self.verify() {
            return Err(AssemblerError::IncompletePacketCollection);
        }

        // Gather references to packets in proper order
        let mut packets = self.packets.values().collect::<Vec<&Packet>>();
        packets.sort_unstable_by_key(|p| p.metadata.index);

        // Collect packet data into one unified binary representation
        // TODO: Improve by NOT cloning data
        let data: Vec<u8> = packets.iter().flat_map(|p| p.data.clone()).collect();

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::super::Metadata;
    use super::*;

    fn helper_new_empty_packet(id: u32, index: u32, is_last: bool) -> Packet {
        Packet {
            metadata: Metadata { id, index, is_last },
            data: vec![],
        }
    }

    #[cfg(test)]
    mod add_packet {
        use super::*;

        #[test]
        fn fails_if_packet_already_exists() {
            let mut a = Assembler::new();
            let index = 999;

            // Add first packet successfully
            let result = a.add_packet(helper_new_empty_packet(0, index, false));
            assert_eq!(
                result.is_ok(),
                true,
                "Expected success for adding first packet, but got {}",
                result.unwrap_err(),
            );

            // Fail if adding packet with same index
            match a
                .add_packet(helper_new_empty_packet(0, index, false))
                .unwrap_err()
            {
                AssemblerError::PacketExists(existing_index) => {
                    assert_eq!(existing_index, index, "Unexpected index returned in error")
                }
                e => panic!("Unexpected error {} received", e),
            }
        }

        #[test]
        fn fails_if_adding_packet_beyond_last() {
            let mut a = Assembler::new();

            // Add first packet successfully
            let result = a.add_packet(helper_new_empty_packet(0, 0, true));
            assert_eq!(
                result.is_ok(),
                true,
                "Expected success for adding first packet, but got {}",
                result.unwrap_err(),
            );

            // Fail if adding packet after final packet
            match a
                .add_packet(helper_new_empty_packet(0, 1, false))
                .unwrap_err()
            {
                AssemblerError::PacketBeyondLastIndex(index, last_index) => {
                    assert_eq!(index, 1, "Beyond index was not one");
                    assert_eq!(last_index, 0, "Last index was not zero");
                }
                e => panic!("Unexpected error {} received", e),
            }
        }

        #[test]
        fn fails_if_packet_does_not_have_same_id() {
            let mut a = Assembler::new();
            let id = 999;

            // Add first packet successfully
            let result = a.add_packet(helper_new_empty_packet(id, 0, false));
            assert_eq!(
                result.is_ok(),
                true,
                "Expected success for adding first packet, but got {}",
                result.unwrap_err(),
            );

            // Fail if adding packet after final packet
            match a
                .add_packet(helper_new_empty_packet(id + 1, 1, false))
                .unwrap_err()
            {
                AssemblerError::PacketHasDifferentId(actual_id, expected_id) => {
                    assert_eq!(actual_id, id + 1, "Actual id was different than provided");
                    assert_eq!(expected_id, id, "Expected id was different from tracked");
                }
                e => panic!("Unexpected error {} received", e),
            }
        }

        #[test]
        fn fails_if_last_packet_already_added() {
            let mut a = Assembler::new();

            // Make the second packet (index) be the last packet
            let result = a.add_packet(helper_new_empty_packet(0, 1, true));
            assert_eq!(
                result.is_ok(),
                true,
                "Expected success for adding first packet, but got {}",
                result.unwrap_err(),
            );

            // Fail if making the first packet (index) be the last packet
            // when we already have a last packet
            match a
                .add_packet(helper_new_empty_packet(0, 0, true))
                .unwrap_err()
            {
                AssemblerError::FinalPacketAlreadyExists(actual_last_index) => {
                    assert_eq!(
                        actual_last_index, 1,
                        "Last packet index different than expected"
                    );
                }
                e => panic!("Unexpected error {} received", e),
            }
        }
    }

    #[cfg(test)]
    mod verify {
        use super::*;

        #[test]
        fn yields_false_if_empty() {
            let a = Assembler::new();
            assert_eq!(a.verify(), false);
        }

        #[test]
        fn yields_false_if_missing_last_packet() {
            let mut a = Assembler::new();

            // Add first packet (index 0), still needing final packet
            let _ = a.add_packet(helper_new_empty_packet(0, 0, false));

            assert_eq!(a.verify(), false);
        }

        #[test]
        fn yields_false_if_missing_first_packet() {
            let mut a = Assembler::new();

            // Add packet at end (index 1), still needing first packet
            assert_eq!(
                a.add_packet(helper_new_empty_packet(0, 1, true)).is_ok(),
                true,
                "Unexpectedly failed to add a new packet",
            );

            assert_eq!(a.verify(), false);
        }

        #[test]
        fn yields_false_if_missing_inbetween_packet() {
            let mut a = Assembler::new();

            // Add packet at beginning (index 0)
            assert_eq!(
                a.add_packet(helper_new_empty_packet(0, 0, false)).is_ok(),
                true,
                "Unexpectedly failed to add a new packet",
            );

            // Add packet at end (index 2)
            assert_eq!(
                a.add_packet(helper_new_empty_packet(0, 2, true)).is_ok(),
                true,
                "Unexpectedly failed to add a new packet",
            );

            assert_eq!(a.verify(), false);
        }

        #[test]
        fn yields_true_if_have_all_packets() {
            let mut a = Assembler::new();

            assert_eq!(
                a.add_packet(helper_new_empty_packet(0, 0, true)).is_ok(),
                true,
                "Unexpectedly failed to add a new packet",
            );

            assert_eq!(a.verify(), true);
        }
    }

    #[cfg(test)]
    mod assemble {
        use super::*;

        #[test]
        fn fails_if_not_verified() {
            let a = Assembler::new();

            let result = a.assemble();

            match result.unwrap_err() {
                AssemblerError::IncompletePacketCollection => (),
                e => panic!("Unexpected error {} received", e),
            }
        }

        #[test]
        fn yields_data_from_single_packet_if_complete() {
            let mut a = Assembler::new();
            let data: Vec<u8> = vec![1, 2, 3];

            // Try a single packet and collecting data
            let _ = a.add_packet(Packet {
                metadata: Metadata {
                    id: 0,
                    index: 0,
                    is_last: true,
                },
                data: data.clone(),
            });

            let collected_data = a.assemble().unwrap();
            assert_eq!(data, collected_data);
        }

        #[test]
        fn yields_combined_data_from_multiple_packets_if_complete() {
            let mut a = Assembler::new();
            let data: Vec<u8> = vec![1, 2, 3, 4, 5];

            // Try a multiple packets and collecting data
            let _ = a.add_packet(Packet {
                metadata: Metadata {
                    id: 0,
                    index: 2,
                    is_last: true,
                },
                data: data[3..].to_vec(),
            });
            let _ = a.add_packet(Packet {
                metadata: Metadata {
                    id: 0,
                    index: 0,
                    is_last: false,
                },
                data: data[0..1].to_vec(),
            });
            let _ = a.add_packet(Packet {
                metadata: Metadata {
                    id: 0,
                    index: 1,
                    is_last: false,
                },
                data: data[1..3].to_vec(),
            });

            let collected_data = a.assemble().unwrap();
            assert_eq!(data, collected_data);
        }
    }
}
