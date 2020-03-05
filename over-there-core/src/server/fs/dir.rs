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

impl LocalDirEntry {
    pub fn path_to_string(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
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
    let metadata = fs::metadata(from.as_ref())
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

pub async fn create(
    path: impl AsRef<Path>,
    create_components: bool,
) -> io::Result<()> {
    if create_components {
        fs::create_dir_all(path).await
    } else {
        fs::create_dir(path).await
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

    #[tokio::test]
    async fn entries_should_yield_error_if_not_a_directory() {
        let result = {
            let file = tempfile::NamedTempFile::new().unwrap();
            entries(file.as_ref()).await
        };

        match result {
            Err(LocalDirEntriesError::ReadDirError(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn entries_should_return_immediate_entries_within_dir() {
        let (dir_path, result) = {
            let dir = tempfile::tempdir().unwrap();

            fs::File::create(dir.as_ref().join("test-file"))
                .await
                .expect("Failed to create file");

            fs::create_dir(dir.as_ref().join("test-dir"))
                .await
                .expect("Failed to create dir");

            let result = entries(dir.as_ref()).await;

            (dir.into_path(), result)
        };

        match result {
            Ok(entries) => {
                assert_eq!(entries.len(), 2, "Unexpected number of entries");

                assert!(
                    entries.contains(&LocalDirEntry {
                        path: dir_path.join("test-file"),
                        is_file: true,
                        is_dir: false,
                        is_symlink: false,
                    }),
                    "No test-file found"
                );

                assert!(
                    entries.contains(&LocalDirEntry {
                        path: dir_path.join("test-dir"),
                        is_file: false,
                        is_dir: true,
                        is_symlink: false,
                    }),
                    "No test-dir found"
                );
            }
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_should_yield_error_if_not_a_directory() {
        let result = {
            let from_file = tempfile::NamedTempFile::new().unwrap();
            let from = from_file.as_ref();
            let to_dir = tempfile::tempdir().unwrap();
            let to = to_dir.as_ref();

            rename(from, to).await
        };

        match result {
            Err(LocalDirRenameError::NotADirectory) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_should_return_success_if_able_to_rename_directory() {
        let result = {
            let from_dir = tempfile::tempdir().unwrap();
            let from = from_dir.as_ref();
            let to_dir = tempfile::tempdir().unwrap();
            let to = to_dir.as_ref();

            rename(from, to).await
        };

        match result {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn create_should_return_success_if_able_to_make_an_empty_directory() {
        let result = {
            let parent_dir = tempfile::tempdir().unwrap();

            create(parent_dir.as_ref().join("test-dir"), false).await
        };

        assert!(result.is_ok(), "Failed unexpectedly: {:?}", result);

        let result = {
            let parent_dir = tempfile::tempdir().unwrap();

            create(parent_dir.as_ref().join("test-dir"), true).await
        };

        assert!(result.is_ok(), "Failed unexpectedly: {:?}", result);
    }

    #[tokio::test]
    async fn create_should_yield_error_if_some_components_dont_exist_and_flag_not_set(
    ) {
        let result = {
            let parent_dir = tempfile::tempdir().unwrap();
            let new_dir = parent_dir.as_ref().join(
                ["does", "not", "exist"]
                    .iter()
                    .collect::<PathBuf>()
                    .as_path(),
            );

            create(new_dir, false).await
        };

        assert!(result.is_err(), "Unexpectedly succeeded: {:?}", result);
    }

    #[tokio::test]
    async fn create_should_return_success_if_able_to_make_nested_empty_directory(
    ) {
        let parent_dir = tempfile::tempdir().unwrap();

        create(parent_dir.as_ref().join("test-dir"), false)
            .await
            .expect("Failed to create directory");

        create(parent_dir.as_ref().join("test-dir"), true)
            .await
            .expect("Failed to create directory");
    }

    #[tokio::test]
    async fn remove_should_yield_error_if_not_a_directory() {
        let result = {
            let file = tempfile::NamedTempFile::new().unwrap();
            remove(file.as_ref(), false).await
        };

        match result {
            Err(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_should_return_success_if_able_to_remove_empty_directory() {
        // Remove an empty directory with non-empty flag not set
        let result = {
            let dir = tempfile::tempdir().unwrap();
            remove(dir.as_ref(), false).await
        };

        match result {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }

        // Remove an empty directory with non-empty flag set
        let result = {
            let dir = tempfile::tempdir().unwrap();
            remove(dir.as_ref(), true).await
        };

        match result {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_should_yield_error_if_removing_nonempty_directory_and_flag_not_set(
    ) {
        let result = {
            let dir = tempfile::tempdir().unwrap();

            fs::File::create(dir.as_ref().join("test-file"))
                .await
                .expect("Failed to create file");

            remove(dir.as_ref(), false).await
        };

        match result {
            Err(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_should_return_success_if_able_to_remove_nonempty_directory_if_flag_set(
    ) {
        let result = {
            let dir = tempfile::tempdir().unwrap();

            fs::File::create(dir.as_ref().join("test-file"))
                .await
                .expect("Failed to create file");

            remove(dir.as_ref(), true).await
        };

        match result {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }
}
