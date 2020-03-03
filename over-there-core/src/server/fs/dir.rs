use over_there_derive::Error;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Error)]
pub enum LocalDirRenameError {
    NotADirectory,
    FailedToGetMetadata(io::Error),
    IoError(io::Error),
}

#[derive(Debug, Error)]
pub enum LocalDirEntriesError {
    ReadDirError(io::Error),
    NextEntryError(io::Error),
    FileTypeError(io::Error),
}

impl Into<io::Error> for LocalDirEntriesError {
    fn into(self: Self) -> io::Error {
        match self {
            Self::ReadDirError(x) => x,
            Self::NextEntryError(x) => x,
            Self::FileTypeError(x) => x,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalDirEntry {
    pub path: PathBuf,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

pub async fn entries(
    path: impl AsRef<Path>,
) -> Result<Vec<LocalDirEntry>, LocalDirEntriesError> {
    let mut entries = Vec::new();
    let mut dir_stream = fs::read_dir(path)
        .await
        .map_err(LocalDirEntriesError::ReadDirError)?;
    while let Some(entry) = dir_stream
        .next_entry()
        .await
        .map_err(LocalDirEntriesError::NextEntryError)?
    {
        let file_type = entry
            .file_type()
            .await
            .map_err(LocalDirEntriesError::FileTypeError)?;
        entries.push(LocalDirEntry {
            path: entry.path(),
            is_file: file_type.is_file(),
            is_dir: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        });
    }
    Ok(entries)
}

pub async fn rename(
    from: impl AsRef<Path>,
    to: impl AsRef<Path>,
) -> Result<(), LocalDirRenameError> {
    let metadata = fs::metadata(from)
        .await
        .map_err(LocalDirRenameError::FailedToGetMetadata)?;

    if metadata.is_dir() {
        fs::rename(from, to)
            .await
            .map_err(LocalDirRenameError::IoError)
    } else {
        Err(LocalDirRenameError::NotADirectory)
    }
}

pub async fn remove(path: impl AsRef<Path>, non_empty: bool) -> io::Result<()> {
    if non_empty {
        fs::remove_dir_all(path).await
    } else {
        fs::remove_dir(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dir_entries_should_yield_error_if_unable_to_read_dir() {
        unimplemented!();
    }

    #[test]
    fn local_dir_entries_should_return_immediate_entries_within_dir() {
        unimplemented!();
    }
}
