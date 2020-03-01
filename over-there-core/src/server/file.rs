use over_there_derive::Error;
use rand::{rngs::OsRng, RngCore};
use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

#[derive(Debug, Error)]
pub enum LocalFileWriteError {
    SigMismatch,
    IoError(LocalFileWriteIoError),
}

#[derive(Debug, Error)]
pub enum LocalFileWriteIoError {
    SeekError(io::Error),
    SetLenError(io::Error),
    WriteAllError(io::Error),
    FlushError(io::Error),
}

impl Into<io::Error> for LocalFileWriteIoError {
    fn into(self: Self) -> io::Error {
        match self {
            Self::SeekError(x) => x,
            Self::SetLenError(x) => x,
            Self::WriteAllError(x) => x,
            Self::FlushError(x) => x,
        }
    }
}

#[derive(Debug, Error)]
pub enum LocalFileReadError {
    SigMismatch,
    IoError(LocalFileReadIoError),
}

#[derive(Debug, Error)]
pub enum LocalFileReadIoError {
    SeekError(io::Error),
    ReadToEndError(io::Error),
}
impl Into<io::Error> for LocalFileReadIoError {
    fn into(self: Self) -> io::Error {
        match self {
            Self::SeekError(x) => x,
            Self::ReadToEndError(x) => x,
        }
    }
}

#[derive(Debug)]
pub struct LocalFile {
    /// Represents a unique id with which to lookup the file
    pub(crate) id: u32,

    /// Represents a unique signature that acts as a barrier to prevent
    /// unexpected operations on the file from a client with an outdated
    /// understanding of the file
    pub(crate) sig: u32,

    /// Represents an underlying file descriptor with which we can read,
    /// write, and perform other operations
    pub(crate) file: File,

    /// Represents the absolute path to the file; any movement
    /// of the file will result in changing the path
    pub(crate) path: PathBuf,
}

impl LocalFile {
    pub(crate) fn new(file: File, path: PathBuf) -> Self {
        let id = OsRng.next_u32();
        let sig = OsRng.next_u32();

        Self {
            id,
            sig,
            file,
            path,
        }
    }

    pub async fn open(
        path: impl AsRef<Path>,
        create: bool,
        write: bool,
        read: bool,
    ) -> io::Result<Self> {
        match OpenOptions::new()
            .create(create)
            .write(write)
            .read(read)
            .open(&path)
            .await
        {
            Ok(file) => {
                let cpath = fs::canonicalize(path).await?;
                Ok(Self::new(file, cpath))
            }
            Err(x) => Err(x),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn sig(&self) -> u32 {
        self.sig
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Reads all contents of file from beginning to end
    pub async fn read_all(
        &mut self,
        sig: u32,
    ) -> Result<Vec<u8>, LocalFileReadError> {
        if self.sig != sig {
            return Err(LocalFileReadError::SigMismatch);
        }

        let mut buf = Vec::new();

        self.file
            .seek(SeekFrom::Start(0))
            .await
            .map_err(LocalFileReadIoError::SeekError)
            .map_err(LocalFileReadError::IoError)?;

        self.file
            .read_to_end(&mut buf)
            .await
            .map_err(LocalFileReadIoError::ReadToEndError)
            .map_err(LocalFileReadError::IoError)?;

        Ok(buf)
    }

    /// Overwrites contents of file with provided contents
    pub async fn write_all(
        &mut self,
        sig: u32,
        buf: &[u8],
    ) -> Result<(), LocalFileWriteError> {
        if self.sig != sig {
            return Err(LocalFileWriteError::SigMismatch);
        }

        // Update our sig before we even touch the file so we guarantee
        // that any modification (even partial) is reflected as a change
        self.sig = OsRng.next_u32();

        self.file
            .seek(SeekFrom::Start(0))
            .await
            .map_err(LocalFileWriteIoError::SeekError)
            .map_err(LocalFileWriteError::IoError)?;

        self.file
            .set_len(0)
            .await
            .map_err(LocalFileWriteIoError::SetLenError)
            .map_err(LocalFileWriteError::IoError)?;

        self.file
            .write_all(buf)
            .await
            .map_err(LocalFileWriteIoError::WriteAllError)
            .map_err(LocalFileWriteError::IoError)?;

        self.file
            .flush()
            .await
            .map_err(LocalFileWriteIoError::FlushError)
            .map_err(LocalFileWriteError::IoError)
    }
}
