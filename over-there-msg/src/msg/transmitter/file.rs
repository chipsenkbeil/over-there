use super::Msg;
use super::{MsgTransmitter, TransmitterError};
use over_there_crypto::Bicrypter;
use over_there_transport::Transmitter;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Duration;

pub struct FileMsgTransmitter {
    pub in_file: File,
    pub out_file: File,
    msg_transmitter: MsgTransmitter,
}

impl FileMsgTransmitter {
    /// 1KB read/write at a time
    pub const MAX_FILE_TRANSMIT_CHUNK_SIZE: usize = 1024;

    pub fn new(in_file: File, out_file: File, msg_transmitter: MsgTransmitter) -> Self {
        FileMsgTransmitter {
            in_file,
            out_file,
            msg_transmitter,
        }
    }

    pub fn from_files(
        in_file: File,
        out_file: File,
        cache_capacity: usize,
        cache_duration: Duration,
        bicrypter: Box<dyn Bicrypter>,
    ) -> Self {
        let transmitter = Transmitter::new(
            Self::MAX_FILE_TRANSMIT_CHUNK_SIZE,
            cache_capacity,
            cache_duration,
            bicrypter,
        );
        let msg_transmitter = MsgTransmitter::new(transmitter);
        Self::new(in_file, out_file, msg_transmitter)
    }

    /// Sends a message using the underlying stream
    /// NOTE: This will cause problems if the msg is larger than our max
    ///       size per packet as each call to send underneath will reset
    ///       to the beginning of the file and overwrite the previous chunk
    pub fn send(&mut self, msg: Msg) -> Result<(), TransmitterError> {
        let mut f = &self.out_file;
        self.msg_transmitter.send(msg, |data| {
            // Clear any existing content in file
            f.set_len(0)?;

            // Start at the beginning so we write properly
            f.seek(SeekFrom::Start(0))?;

            // Ensure all data is placed in file
            f.write_all(&data)?;
            f.flush()
        })
    }

    /// Receives data from the underlying stream, yielding a message if
    /// the final packet has been received
    /// NOTE: This will cause problems if the file contains more than one
    ///       chunk of data as it attempts to read the entire file and
    ///       treat it as one chunk rather than multiple
    pub fn recv(&mut self) -> Result<Option<Msg>, TransmitterError> {
        let mut f = &self.in_file;
        self.msg_transmitter.recv(|buf| {
            // Start at the beginning so we read properly
            f.seek(SeekFrom::Start(0))?;

            // Read full file
            let mut v = Vec::new();
            let size = f.read_to_end(&mut v)?;

            // Copy as much of full file into buffer as we can
            let l = std::cmp::min(size, buf.len());
            buf[..l].clone_from_slice(&v);
            Ok(size)
        })
    }
}
