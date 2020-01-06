use crate::packet::{Metadata, Packet, PacketType};
use over_there_crypto::Nonce;
use over_there_derive::Error;
use over_there_sign::Authenticator;

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
    DesiredChunkSizeTooSmall(usize, usize),
    FailedToSignPacket,
}

pub(crate) struct Disassembler {}

impl Disassembler {
    pub fn make_packets_from_data<A: Authenticator>(
        info: DisassembleInfo<A>,
    ) -> Result<Vec<Packet>, DisassemblerError> {
        let DisassembleInfo {
            id,
            nonce,
            desired_chunk_size,
            authenticator,
            data,
        } = info;

        // We assume that we have a desired chunk size that can fit our
        // metadata and data reasonably
        if desired_chunk_size <= Packet::metadata_size() {
            return Err(DisassemblerError::DesiredChunkSizeTooSmall(
                desired_chunk_size,
                Packet::metadata_size() + 1,
            ));
        }

        // Determine the size of each chunk of data, factoring in our desired
        // chunk size and the space needed for our metadata
        let chunk_size = desired_chunk_size - Packet::metadata_size();

        // Break out our data into chunks that we will place into different
        // packets to reassemble later
        let chunks = data.chunks(chunk_size as usize);

        // Construct the packets, using the single id to associate all of
        // them together and linking each to an individual position in the
        // collective using the chunks
        let mut packets = chunks
            .enumerate()
            .map(|(index, chunk)| {
                let metadata = Metadata {
                    id,
                    index: index as u32,
                    r#type: PacketType::NotFinal,
                };
                metadata.to_vec().map(|md| {
                    let sig = authenticator.sign(&[md, chunk.to_vec()].concat());
                    Packet::new(metadata, sig, chunk.to_vec())
                })
            })
            .collect::<Result<Vec<Packet>, _>>()
            .map_err(|_| DisassemblerError::FailedToSignPacket)?;

        // Modify the last packet to be final
        if let Some(p) = packets.last_mut() {
            let metadata = Metadata {
                id: p.id(),
                index: p.index(),
                r#type: PacketType::Final { nonce },
            };
            let sig = authenticator.sign(
                &[
                    metadata
                        .to_vec()
                        .map_err(|_| DisassemblerError::FailedToSignPacket)?,
                    p.data().to_vec(),
                ]
                .concat(),
            );
            *p = Packet::new(metadata, sig, p.data().to_vec());
        }

        Ok(packets)
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
        let chunk_size = Packet::metadata_size();
        let err = Disassembler::make_packets_from_data(DisassembleInfo {
            id: 0,
            nonce: None,
            data: &vec![1, 2, 3],
            desired_chunk_size: chunk_size,
            authenticator: &NoopAuthenticator,
        })
        .unwrap_err();

        match err {
            DisassemblerError::DesiredChunkSizeTooSmall(size, min_size) => {
                assert_eq!(size, chunk_size);
                assert_eq!(min_size, Packet::metadata_size() + 1);
            }
            x => panic!("Unexpected error: {:?}", x),
        }
    }

    #[test]
    fn produces_single_packet_with_data() {
        let id = 12345;
        let data: Vec<u8> = vec![1, 2];
        let nonce = Some(Nonce::Nonce128Bits(nonce::new_128bit_nonce()));

        // Make it so all the data fits in one packet
        let chunk_size = Packet::metadata_size() + data.len();

        let packets = Disassembler::make_packets_from_data(DisassembleInfo {
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
        let nonce = Some(Nonce::Nonce128Bits(nonce::new_128bit_nonce()));

        // Make it so not all of the data fits in one packet
        let chunk_size = Packet::metadata_size() + 2;

        let packets = Disassembler::make_packets_from_data(DisassembleInfo {
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
}
