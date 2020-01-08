use crate::packet::{Metadata, Packet, PacketType};
use over_there_crypto::Nonce;
use over_there_derive::Error;
use over_there_sign::Authenticator;
use std::collections::HashMap;

pub struct DisassembleInfo<'data, 'a, A: Authenticator> {
    /// ID used to group created packets together
    pub id: u32,

    /// Nonce used to encrypt provided data; if None, implies there was no encryption
    pub nonce: Option<Nonce>,

    /// Desired maximum size of each packet (including metadata)
    pub desired_chunk_size: usize,

    /// Key used to generate signatures
    pub authenticator: &'a A,

    /// The data to build packets around; encryption should have already happened
    /// by this point
    pub data: &'data [u8],
}

#[derive(Debug, Error)]
pub enum DisassemblerError {
    DesiredChunkSizeTooSmall,
    FailedToEstimatePacketSize,
    FailedToSignPacket,
}

pub(crate) struct Disassembler {
    packet_overhead_size_cache: HashMap<String, usize>,
}

impl Disassembler {
    pub fn new() -> Self {
        Self {
            packet_overhead_size_cache: HashMap::new(),
        }
    }

    pub fn make_packets_from_data<A: Authenticator>(
        &mut self,
        info: DisassembleInfo<A>,
    ) -> Result<Vec<Packet>, DisassemblerError> {
        let DisassembleInfo {
            id,
            nonce,
            desired_chunk_size,
            authenticator,
            data,
        } = info;

        // Determine overhead needed to produce packet with desired data size
        let non_final_overhead_size = self
            .cached_estimate_packet_overhead_size(
                desired_chunk_size,
                PacketType::NotFinal,
                authenticator,
            )
            .map_err(|_| DisassemblerError::FailedToEstimatePacketSize)?;

        let final_overhead_size = self
            .cached_estimate_packet_overhead_size(
                desired_chunk_size,
                PacketType::Final { nonce },
                authenticator,
            )
            .map_err(|_| DisassemblerError::FailedToEstimatePacketSize)?;

        // If the packet size would be so big that the overhead is at least
        // as large as our desired total byte stream (chunk) size, we will
        // exit because we cannot send packets without violating the requirement
        if non_final_overhead_size >= desired_chunk_size
            || final_overhead_size >= desired_chunk_size
        {
            return Err(DisassemblerError::DesiredChunkSizeTooSmall);
        }

        // Compute the data size for a non-final and final packet
        let non_final_chunk_size = desired_chunk_size - non_final_overhead_size;
        let final_chunk_size = desired_chunk_size - final_overhead_size;

        // Construct the packets, using the single id to associate all of
        // them together and linking each to an individual position in the
        // collective using the chunks
        let mut packets = Vec::new();
        let mut i = 0;
        while i < data.len() {
            // Chunk length is determined by this logic:
            // 1. If we have room in the final packet for the remaining data,
            //    store it in the final packet
            // 2. If we have so much data left that it won't fit in the final
            //    packet and it won't fit in a non-final packet, we store N
            //    bytes into a non-final packet where N is the capable size
            //    of a non-final packet data section
            // 3. If we have so much data left that it won't fit in the final
            //    packet but it will fit entirely in a non-final packet, we
            //    store N bytes into a non-final packet where N is the capable
            //    size of the final packet data section
            let can_fit_all_in_final_packet = i + final_chunk_size >= data.len();
            let can_fit_all_in_non_final_packet = i + non_final_chunk_size >= data.len();
            let chunk_size = if can_fit_all_in_final_packet || can_fit_all_in_non_final_packet {
                final_chunk_size
            } else {
                non_final_chunk_size
            };

            // Ensure chunk size does not exceed our remaining data
            let chunk_size = std::cmp::min(chunk_size, data.len() - i);

            // Grab our chunk of data to store into a packet
            let chunk = &data[i..i + chunk_size];

            // Construct the packet based on whether or not is final
            let packet = Self::make_new_packet(
                id,
                packets.len() as u32,
                if can_fit_all_in_final_packet {
                    PacketType::Final { nonce }
                } else {
                    PacketType::NotFinal
                },
                chunk,
                authenticator,
            )
            .map_err(|_| DisassemblerError::FailedToSignPacket)?;

            // Store packet in our collection
            packets.push(packet);

            // Move our pointer by N bytes
            i += chunk_size;
        }

        Ok(packets)
    }

    /// Creates a new packet and signs it using the given authenticator
    fn make_new_packet(
        id: u32,
        index: u32,
        r#type: PacketType,
        data: &[u8],
        authenticator: &dyn Authenticator,
    ) -> Result<Packet, rmp_serde::encode::Error> {
        let metadata = Metadata { id, index, r#type };
        metadata.to_vec().map(|md| {
            let sig = authenticator.sign(&[md, data.to_vec()].concat());
            Packet::new(metadata, sig, data.to_vec())
        })
    }

    fn cached_estimate_packet_overhead_size(
        &mut self,
        desired_data_size: usize,
        r#type: PacketType,
        authenticator: &dyn Authenticator,
    ) -> Result<usize, rmp_serde::encode::Error> {
        // Calculate key to use for cache
        // TODO: Convert authenticator into part of the key? Is this necessary?
        let key = format!("{}{:?}", desired_data_size, r#type);

        // Check if we have a cached value and, if so, use it
        if let Some(value) = self.packet_overhead_size_cache.get(&key) {
            return Ok(*value);
        }

        // Otherwise, estimate the packet size, cache it, and return it
        let overhead_size =
            Self::estimate_packet_overhead_size(desired_data_size, r#type, authenticator)?;
        self.packet_overhead_size_cache.insert(key, overhead_size);
        Ok(overhead_size)
    }

    pub(crate) fn estimate_packet_overhead_size(
        desired_data_size: usize,
        r#type: PacketType,
        authenticator: &dyn Authenticator,
    ) -> Result<usize, rmp_serde::encode::Error> {
        println!(
            "Request packet size for desired data size of {} and type {:?}",
            desired_data_size, r#type
        );
        let packet_size = Self::estimate_packet_size(desired_data_size, r#type, authenticator)?;
        println!("It is {}", packet_size);

        // Figure out how much overhead is needed to fit the data into the packet
        // NOTE: If for some reason the packet -> msgpack has optimized the
        //       byte stream so well that it is smaller than the provided
        //       data, we will assume no overhead
        Ok(if packet_size > desired_data_size {
            packet_size - desired_data_size
        } else {
            0
        })
    }

    fn estimate_packet_size(
        desired_data_size: usize,
        r#type: PacketType,
        authenticator: &dyn Authenticator,
    ) -> Result<usize, rmp_serde::encode::Error> {
        // Produce random fake data to avoid any byte sequencing
        let fake_data: Vec<u8> = (0..desired_data_size)
            .map(|_| rand::random::<u8>())
            .collect();

        // Produce a fake packet whose data fills the entire size, and then
        // see how much larger it is and use that as the overhead cost
        //
        // NOTE: This is a rough estimate and requires an entire serialization,
        //       but is the most straightforward way I can think of unless
        //       serde offers some form of size hinting for msgpack specifically
        Disassembler::make_new_packet(
            u32::max_value(),
            u32::max_value(),
            r#type,
            &fake_data,
            authenticator,
        )?
        .to_vec()
        .map(|v| v.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use over_there_crypto::nonce;
    use over_there_sign::NoopAuthenticator;

    #[test]
    fn fails_if_desired_chunk_size_is_too_low() {
        // Needs to accommodate metadata & data, which this does not
        let chunk_size = 1;
        let err = Disassembler::new()
            .make_packets_from_data(DisassembleInfo {
                id: 0,
                nonce: None,
                data: &vec![1, 2, 3],
                desired_chunk_size: chunk_size,
                authenticator: &NoopAuthenticator,
            })
            .unwrap_err();

        match err {
            DisassemblerError::DesiredChunkSizeTooSmall => (),
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn produces_single_packet_with_data() {
        let id = 12345;
        let data: Vec<u8> = vec![1, 2];
        let nonce = Some(Nonce::from(nonce::new_128bit_nonce()));

        // Make it so all the data fits in one packet
        let chunk_size = 1000;

        let packets = Disassembler::new()
            .make_packets_from_data(DisassembleInfo {
                id,
                nonce,
                data: &data,
                desired_chunk_size: chunk_size,
                authenticator: &NoopAuthenticator,
            })
            .unwrap();
        assert_eq!(packets.len(), 1, "More than one packet produced");

        let p = &packets[0];
        assert_eq!(p.id(), id, "ID not properly set on packet");
        assert_eq!(p.index(), 0, "Unexpected index for single packet");
        assert_eq!(
            p.is_final(),
            true,
            "Single packet not marked as last packet"
        );
        assert_eq!(p.data(), &data);
    }

    #[test]
    fn produces_multiple_packets_with_data() {
        let id = 67890;
        let data: Vec<u8> = vec![1, 2, 3];
        let nonce = Some(Nonce::from(nonce::new_128bit_nonce()));

        // Calculate the bigger of the two overhead sizes (final packet)
        // and ensure that we can only fit the last element in it
        let overhead_size = Disassembler::estimate_packet_overhead_size(
            /* data size */ 1,
            PacketType::Final { nonce },
            &NoopAuthenticator,
        )
        .unwrap();
        let chunk_size = overhead_size + 2;

        let packets = Disassembler::new()
            .make_packets_from_data(DisassembleInfo {
                id,
                nonce,
                data: &data,
                desired_chunk_size: chunk_size,
                authenticator: &NoopAuthenticator,
            })
            .unwrap();
        assert_eq!(packets.len(), 2, "Unexpected number of packets");

        // Check data quality of first packet
        let p1 = packets.get(0).unwrap();
        assert_eq!(p1.id(), id, "ID not properly set on first packet");
        assert_eq!(p1.index(), 0, "First packet not marked with index 0");
        assert_eq!(
            p1.is_final(),
            false,
            "Non-final packet unexpectedly marked as last"
        );
        assert_eq!(&p1.data()[..], &data[0..2]);

        // Check data quality of second packet
        let p2 = packets.get(1).unwrap();
        assert_eq!(p2.id(), id, "ID not properly set on second packet");
        assert_eq!(p2.index(), 1, "Last packet not marked with correct index");
        assert_eq!(p2.is_final(), true, "Last packet not marked as final");
        assert_eq!(&p2.data()[..], &data[2..]);
    }

    #[test]
    fn produces_multiple_packets_respecting_size_constraints() {
        let id = 67890;
        let nonce = Some(Nonce::from(nonce::new_128bit_nonce()));

        // Make it so not all of the data fits in one packet
        //
        // NOTE: Make sure we make large enough chunks so msgpack
        //       serialization needs more bytes; use 100k of memory to
        //       spread out packets
        //
        let data: Vec<u8> = [0; 100000].to_vec();
        let chunk_size = 512;

        let packets = Disassembler::new()
            .make_packets_from_data(DisassembleInfo {
                id,
                nonce,
                data: &data,
                desired_chunk_size: chunk_size,
                authenticator: &NoopAuthenticator,
            })
            .unwrap();

        // All packets should be no larger than chunk size
        for (i, p) in packets.iter().enumerate() {
            let actual_size = p.to_vec().unwrap().len();
            assert!(
                actual_size <= chunk_size,
                "Serialized packet {}/{} was {} bytes instead of max size of {}",
                i + 1,
                packets.len(),
                actual_size,
                chunk_size
            );
        }
    }
}
