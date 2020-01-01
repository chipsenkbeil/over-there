use crate::packet::Packet;
use over_there_derive::*;

#[derive(Debug, Error)]
pub enum DisassemblerError {
    DesiredChunkSizeTooSmall(usize, usize),
}

pub(crate) struct Disassembler {}

impl Disassembler {
    pub fn make_packets_from_data(
        id: u32,
        data: Vec<u8>,
        desired_chunk_size: usize,
    ) -> Result<Vec<Packet>, DisassemblerError> {
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
        let total_chunks = chunks.len();

        // Construct the packets, using the single id to associate all of
        // them together and linking each to an individual position in the
        // collective using the chunks
        let packets = chunks
            .enumerate()
            .map(|(index, chunk)| {
                Packet::new(id, index as u32, index + 1 == total_chunks, chunk.to_vec())
            })
            .collect::<Vec<Packet>>();
        Ok(packets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fails_if_desired_chunk_size_is_too_low() {
        // Needs to accommodate metadata & data, which this does not
        let chunk_size = Packet::metadata_size();

        let DisassemblerError::DesiredChunkSizeTooSmall(size, min_size) =
            Disassembler::make_packets_from_data(0, vec![1, 2, 3], chunk_size).unwrap_err();
        assert_eq!(size, chunk_size);
        assert_eq!(min_size, Packet::metadata_size() + 1);
    }

    #[test]
    fn produces_single_packet_with_data() {
        let id = 12345;
        let data: Vec<u8> = vec![1, 2];

        // Make it so all the data fits in one packet
        let chunk_size = Packet::metadata_size() + data.len();

        let packets = Disassembler::make_packets_from_data(id, data.clone(), chunk_size).unwrap();
        assert_eq!(packets.len(), 1, "More than one packet produced");

        let p = &packets[0];
        assert_eq!(p.id(), id, "ID not properly set on packet");
        assert_eq!(p.index(), 0, "Unexpected index for single packet");
        assert_eq!(p.is_last(), true, "Single packet not marked as last packet");
        assert_eq!(p.data(), &data);
    }

    #[test]
    fn produces_multiple_packets_with_data() {
        let id = 67890;
        let data: Vec<u8> = vec![1, 2, 3];

        // Make it so not all of the data fits in one packet
        let chunk_size = Packet::metadata_size() + 2;

        let packets = Disassembler::make_packets_from_data(id, data.clone(), chunk_size).unwrap();
        assert_eq!(packets.len(), 2, "Unexpected number of packets");

        // Check data quality of first packet
        let p1 = packets.get(0).unwrap();
        assert_eq!(p1.id(), id, "ID not properly set on first packet");
        assert_eq!(p1.index(), 0, "First packet not marked with index 0");
        assert_eq!(
            p1.is_last(),
            false,
            "Non-final packet unexpectedly marked as last"
        );
        assert_eq!(&p1.data()[..], &data[0..2]);

        // Check data quality of second packet
        let p2 = packets.get(1).unwrap();
        assert_eq!(p2.id(), id, "ID not properly set on second packet");
        assert_eq!(p2.index(), 1, "Last packet not marked with correct index");
        assert_eq!(p2.is_last(), true, "Last packet not marked as final");
        assert_eq!(&p2.data()[..], &data[2..]);
    }
}
