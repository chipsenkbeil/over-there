use std::error::Error;

#[derive(Debug)]
pub enum DisassemblerError {
    DesiredChunkSizeTooSmall(u32, u32),
}

impl std::fmt::Display for DisassemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            DisassemblerError::DesiredChunkSizeTooSmall(size, min_size) => write!(
                f,
                "Desired chunk size of {} is not {} or greater",
                size, min_size
            ),
        }
    }
}

impl Error for DisassemblerError {}

#[derive(Debug)]
pub enum AssemblerError {
    PacketExists(u32),
    PacketBeyondLastIndex(u32, u32),
    PacketHasDifferentId(u32, u32),
    FinalPacketAlreadyExists(u32),
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
            AssemblerError::FinalPacketAlreadyExists(final_packet_index) => write!(
                f,
                "Packet at index {} is already marked as the last packet",
                final_packet_index
            ),
            AssemblerError::IncompletePacketCollection => {
                write!(f, "Attempted to assemble without all packets!")
            }
        }
    }
}
