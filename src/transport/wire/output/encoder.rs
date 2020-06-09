use crate::transport::{
    auth::Signer,
    wire::packet::{Metadata, Packet, PacketEncryption, PacketType},
};
use over_there_derive::Error;
use std::collections::HashMap;

pub(crate) struct EncodeArgs<'d, 's, S: Signer> {
    /// ID used to group created packets together
    pub id: u32,

    /// Used to specify the level of encryption to use
    pub encryption: PacketEncryption,

    /// Desired maximum size of each packet (including all overhead like metadata)
    pub max_packet_size: usize,

    /// Key used to generate signatures
    pub signer: &'s S,

    /// The data to build packets around; encryption should have already happened
    /// by this point
    pub data: &'d [u8],
}

#[derive(Debug, Error)]
pub enum EncoderError {
    MaxPacketSizeTooSmall,
    FailedToEstimateDataSize,
    FailedToSignPacket,
}

#[derive(Debug, Clone)]
pub(crate) struct Encoder {
    /// Key = Max Packet Size + PacketType
    /// Value = Max data size to fit in packet without exceeding max packet size
    max_data_size_cache: HashMap<String, usize>,

    /// Key = Data Size + PacketType
    /// Value = Size of packet given some length of data and packet type
    packet_size_cache: HashMap<String, usize>,
}

impl Encoder {
    pub fn encode<S: Signer>(
        &mut self,
        info: EncodeArgs<S>,
    ) -> Result<Vec<Packet>, EncoderError> {
        let EncodeArgs {
            id,
            encryption,
            max_packet_size,
            signer,
            data,
        } = info;

        // Calculate the maximum size of the data section of a packet for
        // a final and not final version
        let not_final_max_data_size = self
            .find_optimal_max_data_size(
                max_packet_size,
                PacketType::NotFinal,
                signer,
            )
            .map_err(|_| EncoderError::FailedToEstimateDataSize)?;
        let final_max_data_size = self
            .find_optimal_max_data_size(
                max_packet_size,
                PacketType::Final { encryption },
                signer,
            )
            .map_err(|_| EncoderError::FailedToEstimateDataSize)?;

        // If a packet type cannot support data of any size, we have requested
        // a max packet size that is too small
        if not_final_max_data_size == 0 || final_max_data_size == 0 {
            return Err(EncoderError::MaxPacketSizeTooSmall);
        }

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
            //    bytes into a non-final packet where N is the max capable size
            //    of a non-final packet data section
            // 3. If we have so much data left that it won't fit in the final
            //    packet but it will fit entirely in a non-final packet, we
            //    store N - 1 bytes where N is the size of the remaining data
            let remaining_data_size = data.len() - i;
            let can_fit_all_in_final_packet =
                i + final_max_data_size >= data.len();
            let can_fit_all_in_not_final_packet =
                i + not_final_max_data_size >= data.len();
            let data_size = if can_fit_all_in_final_packet {
                final_max_data_size
            } else if can_fit_all_in_not_final_packet {
                remaining_data_size - 1
            } else {
                not_final_max_data_size
            };

            // Ensure data section size does not exceed our remaining data
            let data_size = std::cmp::min(data_size, remaining_data_size);

            // Grab our chunk of data to store into a packet
            let chunk = &data[i..i + data_size];

            // Construct the packet based on whether or not is final
            let packet = Self::make_new_packet(
                id,
                packets.len() as u32,
                if can_fit_all_in_final_packet {
                    PacketType::Final { encryption }
                } else {
                    PacketType::NotFinal
                },
                chunk,
                signer,
            )
            .map_err(|_| EncoderError::FailedToSignPacket)?;

            // Store packet in our collection
            packets.push(packet);

            // Move our pointer by N bytes
            i += data_size;
        }

        Ok(packets)
    }

    /// Creates a new packet and signs it using the given authenticator
    fn make_new_packet<S: Signer>(
        id: u32,
        index: u32,
        r#type: PacketType,
        data: &[u8],
        signer: &S,
    ) -> Result<Packet, serde_cbor::Error> {
        let metadata = Metadata { id, index, r#type };
        metadata.to_vec().map(|md| {
            let sig = signer.sign(&[md, data.to_vec()].concat());
            Packet::new(metadata, sig, data.to_vec())
        })
    }

    /// Finds the optimal data size to get as close to a maximum packet size
    /// as possible without exceeding it. Will cache results for faster
    /// performance on future runs.
    fn find_optimal_max_data_size<S: Signer>(
        &mut self,
        max_packet_size: usize,
        r#type: PacketType,
        signer: &S,
    ) -> Result<usize, serde_cbor::Error> {
        // Calculate key to use for cache
        let key = format!("{}{:?}", max_packet_size, r#type);

        // Check if we have a cached value and, if so, use it
        if let Some(value) = self.max_data_size_cache.get(&key) {
            return Ok(*value);
        }

        // Start searching somewhere in the middle of the maximum packet size
        let mut best_data_size = 0;
        let mut data_size = (max_packet_size / 2) + 1;
        loop {
            let packet_size =
                self.estimate_packet_size(data_size, r#type, signer)?;

            // If the data section has reached our maximum packet size exactly,
            // we are done searching
            if packet_size == max_packet_size {
                best_data_size = data_size;
                break;
            // Else, if the packet size would be too big, shrink down
            } else if packet_size > max_packet_size {
                let overflow_size = packet_size - max_packet_size;

                // If the overflow is greater than the data available, shrink
                // down by half (in case the overhead shrinks)
                if overflow_size >= data_size {
                    data_size /= 2;

                    // If the shrinkage would cause the data to be nothing, exit
                    if data_size == 0 {
                        break;
                    }

                // Otherwise, shrink data size by N to see if we can fit
                } else {
                    data_size -= overflow_size;
                }

            // Else, if the packet was smaller than max capacity AND the data
            // that can be fit is better (bigger) than the current best fit,
            // update it
            } else if data_size > best_data_size {
                best_data_size = data_size;
            // Else, we've reached a point where we are no longer finding
            // better sizes, so exit with what we've found so far
            } else {
                break;
            }
        }

        // Cache our best size for the data section given max packet size
        self.max_data_size_cache.insert(key, best_data_size);
        Ok(best_data_size)
    }

    /// Calculates the size of a packet comprised of data of length N, caching
    /// the result to avoid expensive computations in the future.
    pub(crate) fn estimate_packet_size<S: Signer>(
        &mut self,
        data_size: usize,
        r#type: PacketType,
        signer: &S,
    ) -> Result<usize, serde_cbor::Error> {
        // Calculate key to use for cache
        let key = format!("{}{:?}", data_size, r#type);

        // Check if we have a cached value and, if so, use it
        if let Some(value) = self.packet_size_cache.get(&key) {
            return Ok(*value);
        }

        // Produce random fake data to avoid any byte sequencing
        let fake_data: Vec<u8> =
            (0..data_size).map(|_| rand::random::<u8>()).collect();

        // Produce a fake packet whose data fills the entire size, and then
        // see how much larger it is and use that as the overhead cost
        //
        // NOTE: This is a rough estimate and requires an entire serialization,
        //       but is the most straightforward way I can think of unless
        //       serde offers some form of size hinting for msgpack/cbor specifically
        let packet_size = Encoder::make_new_packet(
            u32::max_value(),
            u32::max_value(),
            r#type,
            &fake_data,
            signer,
        )?
        .to_vec()?
        .len();

        // Cache the calculated size and return it
        self.packet_size_cache.insert(key, packet_size);
        Ok(packet_size)
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            max_data_size_cache: HashMap::new(),
            packet_size_cache: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::auth::NoopAuthenticator;
    use crate::transport::crypto::{nonce, Nonce};

    #[test]
    fn fails_if_max_packet_size_is_too_low() {
        // Needs to accommodate metadata & data, which this does not
        let chunk_size = 1;
        let err = Encoder::default()
            .encode(EncodeArgs {
                id: 0,
                encryption: PacketEncryption::None,
                data: &vec![1, 2, 3],
                max_packet_size: chunk_size,
                signer: &NoopAuthenticator,
            })
            .unwrap_err();

        match err {
            EncoderError::MaxPacketSizeTooSmall => (),
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn produces_single_packet_with_data() {
        let id = 12345;
        let data: Vec<u8> = vec![1, 2];
        let encryption =
            PacketEncryption::from(Nonce::from(nonce::new_128bit_nonce()));

        // Make it so all the data fits in one packet
        let chunk_size = 1000;

        let packets = Encoder::default()
            .encode(EncodeArgs {
                id,
                encryption,
                data: &data,
                max_packet_size: chunk_size,
                signer: &NoopAuthenticator,
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
        let nonce = Nonce::from(nonce::new_128bit_nonce());
        let mut encoder = Encoder::default();

        // Calculate a packet size where the final packet can only
        // fit a single byte of data to ensure that we get at least
        // one additional packet
        let max_packet_size = encoder
            .estimate_packet_size(
                /* data size */ 1,
                PacketType::Final {
                    encryption: PacketEncryption::from(nonce),
                },
                &NoopAuthenticator,
            )
            .unwrap();

        let packets = encoder
            .encode(EncodeArgs {
                id,
                encryption: PacketEncryption::from(nonce),
                data: &data,
                max_packet_size,
                signer: &NoopAuthenticator,
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
        let encryption =
            PacketEncryption::from(Nonce::from(nonce::new_128bit_nonce()));

        // Make it so not all of the data fits in one packet
        //
        // NOTE: Make sure we make large enough chunks so msgpack
        //       serialization needs more bytes; use 100k of memory to
        //       spread out packets
        //
        let data: Vec<u8> = [0; 100000].to_vec();
        let chunk_size = 512;

        let packets = Encoder::default()
            .encode(EncodeArgs {
                id,
                encryption,
                data: &data,
                max_packet_size: chunk_size,
                signer: &NoopAuthenticator,
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
