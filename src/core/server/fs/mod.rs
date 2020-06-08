mod dir;
mod file;

pub use dir::LocalDirEntry;
pub use file::{
    LocalFile, LocalFileError, LocalFileHandle, LocalFilePermissions,
};

use std::collections::{hash_map::Entry, HashMap};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileSystemManager {
    files: HashMap<u32, LocalFile>,
}

impl Default for FileSystemManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemManager {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Creates a new directory
    pub async fn create_dir(
        &self,
        path: impl AsRef<Path>,
        create_components: bool,
    ) -> io::Result<()> {
        let path = clean_path(path.as_ref()).await;
        dir::create(path, create_components).await
    }

    /// Attempts to rename an entire directory.
    ///
    /// Will fail if there is an open file within the directory on any level.
    pub async fn rename_dir(
        &mut self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> io::Result<()> {
        let from = clean_path(from.as_ref()).await;
        let to = clean_path(to.as_ref()).await;

        self.check_no_open_files(from.as_path())?;

        // No open file is within this directory, so good to attempt to rename
        dir::rename(from.as_path(), to.as_path()).await?;

        Ok(())
    }

    /// Attempts to remove an entire directory, failing if any file is
    /// currently open within the directory.
    pub async fn remove_dir(
        &mut self,
        path: impl AsRef<Path>,
        non_empty: bool,
    ) -> io::Result<()> {
        let path = clean_path(path.as_ref()).await;

        self.check_no_open_files(path.as_path())?;

        // No open file is within this directory, so good to attempt to remove
        dir::remove(path, non_empty).await
    }

    /// Retrieves all entries within the directory `path`.
    ///
    /// This is a non-recursive operation, meaning that it will only yield
    /// the immediate directory entires and not walk through subdirectories
    /// or follow symlinks.
    ///
    /// Will yield an error if there are complications with reading the
    /// directory and its entries.
    pub async fn dir_entries(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<Vec<LocalDirEntry>> {
        let path = clean_path(path.as_ref()).await;

        dir::entries(path).await
    }

    /// Opens a file, creating it if `create` true, using `write` and `read`
    /// for permissions.
    ///
    /// If the file is already open, will return the existing instance. If
    /// permissions differ where the returned file does not have read/write
    /// access and the request asks for it, the current instance of the file
    /// will be closed and a new instance with the same id will be opened with
    /// the new permissions where existing and requested permissions align.
    pub async fn open_file(
        &mut self,
        path: impl AsRef<Path>,
        create: bool,
        write: bool,
        read: bool,
    ) -> io::Result<LocalFileHandle> {
        let path = clean_path(path.as_ref()).await;

        let mut new_permissions = LocalFilePermissions { read, write };
        let mut maybe_id_and_sig = None;

        // TODO: Perform more optimal lookup by filtering down open files
        //       using a path tree?
        let search =
            self.files.values_mut().find(|f| f.path() == path.as_path());

        // If we found a match, check the permissions to see if we can return
        // it or if we need to open a new copy with the proper merged
        // permissions
        if let Some(file) = search {
            let id = file.id();
            let sig = file.sig();
            let permissions = file.permissions();

            // We already have read permission or are not asking for it and
            // we already have write permission or are not asking for it
            if (permissions.read || !read) && (permissions.write || !write) {
                return Ok(file.handle());
            } else {
                // Otherwise, we now need to open a new file pointer with the
                // proper permissions to support both cases and, if successful,
                // close the existing file
                new_permissions.read = permissions.read || read;
                new_permissions.write = permissions.write || write;
                maybe_id_and_sig = Some((id, sig));
            }
        }

        // Open the file with the specified path
        let mut new_file = LocalFile::open(
            path,
            create,
            new_permissions.write,
            new_permissions.read,
        )
        .await?;

        // If we already had a file open with this path, we want to assign
        // the previously-used id and sig
        if let Some((id, sig)) = maybe_id_and_sig {
            new_file.id = id;
            new_file.sig = sig;
        }

        // Insert the file & permissions, overwriting the
        // existing file/permissions
        let handle = new_file.handle();
        self.files.insert(new_file.id(), new_file);

        Ok(handle)
    }

    /// Closes an open file by `handle`.
    ///
    /// Will fail if no file with `handle` id is open, or if the signature
    /// on the file is different than that of `handle`.
    pub fn close_file(
        &mut self,
        handle: LocalFileHandle,
    ) -> io::Result<LocalFile> {
        match self.files.entry(handle.id) {
            Entry::Occupied(x) if x.get().sig == handle.sig => Ok(x.remove()),
            Entry::Occupied(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Signature invalid for file with id {}", handle.id),
            )),
            Entry::Vacant(_) => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No open file with id {}", handle.id),
            )),
        }
    }

    /// Attempts to rename a file at `from` into `to`.
    ///
    /// Will fail if file is open at `from`.
    pub async fn rename_file(
        &mut self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> io::Result<()> {
        let from = clean_path(from.as_ref()).await;
        let to = clean_path(to.as_ref()).await;

        self.check_no_open_files(from.as_path())?;

        file::rename(from.as_path(), to.as_path()).await
    }

    /// Attempts to remove a file, failing if the file is currently open.
    pub async fn remove_file(
        &mut self,
        path: impl AsRef<Path>,
    ) -> io::Result<()> {
        let path = clean_path(path.as_ref()).await;

        self.check_no_open_files(path.as_path())?;

        file::remove(path).await
    }

    /// Represents the total files that are open within the manager
    pub fn file_cnt(&self) -> usize {
        self.files.len()
    }

    /// Looks up an open file by its associated `id`
    pub fn get_mut(&mut self, id: impl Into<u32>) -> Option<&mut LocalFile> {
        match self.files.get_mut(&id.into()) {
            Some(file) => Some(file),
            None => None,
        }
    }

    /// Looks up an open file by its associated `id`
    pub fn get(&self, id: impl Into<u32>) -> Option<&LocalFile> {
        match self.files.get(&id.into()) {
            Some(file) => Some(file),
            None => None,
        }
    }

    /// Determines if a file is open with the specified `id`
    pub fn exists(&self, id: impl Into<u32>) -> bool {
        self.get(id).is_some()
    }

    /// Checks that `path` is not an open file or (if dir) does not contain any
    /// open files managed by the file system manager
    fn check_no_open_files(&self, path: impl AsRef<Path>) -> io::Result<()> {
        for f in self.files.values() {
            if f.path().starts_with(path.as_ref()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "File at {:?} is open and must be closed",
                        f.path()
                    ),
                ));
            }
        }

        Ok(())
    }
}

/// Attempts to canonicalize the path, returning the canonicalized form
/// or the original form if failed.
async fn clean_path(path: impl AsRef<Path>) -> PathBuf {
    tokio::fs::canonicalize(path.as_ref())
        .await
        .ok()
        .unwrap_or_else(|| path.as_ref().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn create_dir_should_yield_error_if_parent_dirs_missing_and_flag_not_set(
    ) {
        let root = tempfile::tempdir().unwrap();
        let fsm = FileSystemManager::new();

        let result =
            fsm.create_dir(root.as_ref().join("some/dir"), false).await;
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn create_dir_should_return_success_if_created_the_path() {
        let root = tempfile::tempdir().unwrap();
        let fsm = FileSystemManager::new();

        let path = root.as_ref().join("test-dir");
        let result = fsm.create_dir(path.as_path(), false).await;
        assert!(
            result.is_ok(),
            "Unexpectedly failed to create dir: {:?}",
            result
        );
        assert!(fs::metadata(path).await.is_ok(), "Directory  missing");

        let path = root.as_ref().join("some/test-dir");
        let result = fsm.create_dir(path.as_path(), true).await;
        assert!(
            result.is_ok(),
            "Unexpectedly failed to create nested dir: {:?}",
            result
        );
        assert!(fs::metadata(path).await.is_ok(), "Directory missing");
    }

    #[tokio::test]
    async fn rename_dir_should_yield_error_if_origin_path_does_not_exist() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let origin = root.as_ref().join("origin");
        let destination = root.as_ref().join("destination");

        match fsm.rename_dir(origin, destination).await {
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::NotFound),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_dir_should_yield_error_if_origin_path_is_not_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        // Make origin a file instead of directory
        let origin_file =
            tempfile::NamedTempFile::new_in(root.as_ref()).unwrap();

        let destination = root.as_ref().join("destination");

        match fsm.rename_dir(origin_file.as_ref(), destination).await {
            Err(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_dir_should_yield_error_if_contains_open_files() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let origin = root.as_ref().join("origin");
        fs::create_dir(origin.as_path()).await.unwrap();

        // Create a file in origin
        let _file1 = fsm
            .open_file(origin.as_path().join("file1"), true, true, true)
            .await
            .unwrap();

        let destination = root.as_ref().join("destination");

        match fsm
            .rename_dir(origin.as_path(), destination.as_path())
            .await
        {
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::InvalidData),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_dir_should_return_success_if_renamed_directory() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let origin = root.as_ref().join("origin");
        fs::create_dir(origin.as_path()).await.unwrap();

        let destination = root.as_ref().join("destination");

        match fsm.rename_dir(origin, destination).await {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn dir_entries_should_yield_error_if_path_not_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let fsm = FileSystemManager::new();

        let file = tempfile::NamedTempFile::new_in(root.as_ref()).unwrap();

        match fsm.dir_entries(file.path()).await {
            Err(x) if x.kind() == io::ErrorKind::Other => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn dir_entries_should_return_a_list_of_immediate_entries_in_a_directory(
    ) {
        let root = tempfile::tempdir().unwrap();
        let fsm = FileSystemManager::new();

        let file = tempfile::NamedTempFile::new_in(root.as_ref()).unwrap();
        let dir = tempfile::tempdir_in(root.as_ref()).unwrap();
        let inner_file = tempfile::NamedTempFile::new_in(dir.as_ref()).unwrap();

        match fsm.dir_entries(root.as_ref()).await {
            Ok(entries) => {
                assert_eq!(
                    entries.len(),
                    2,
                    "Unexpected entry count: {}",
                    entries.len()
                );
                assert!(
                    entries.contains(&LocalDirEntry {
                        path: clean_path(file.as_ref()).await,
                        is_file: true,
                        is_dir: false,
                        is_symlink: false,
                    }),
                    "Missing file"
                );
                assert!(
                    entries.contains(&LocalDirEntry {
                        path: clean_path(dir.as_ref()).await,
                        is_file: false,
                        is_dir: true,
                        is_symlink: false,
                    }),
                    "Missing dir"
                );
                assert!(
                    !entries.contains(&LocalDirEntry {
                        path: clean_path(inner_file.as_ref()).await,
                        is_file: true,
                        is_dir: false,
                        is_symlink: false,
                    }),
                    "Unexpectedly found nested file"
                );
            }
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_dir_should_yield_error_if_directory_not_empty_and_flag_not_set(
    ) {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        // NOTE: Must be kept around so that the file exists when removing dir
        let _file = tempfile::NamedTempFile::new_in(root.as_ref()).unwrap();

        match fsm.remove_dir(root.as_ref(), false).await {
            Err(x) if x.kind() == io::ErrorKind::Other => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_dir_should_yield_error_if_open_files_exist_in_directory() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        fsm.open_file(root.as_ref().join("test-file"), true, true, true)
            .await
            .expect("Failed to open file with manager");

        // Even though we want to remove everything, still cannot do it because
        // a local file is open
        match fsm.remove_dir(root.as_ref(), true).await {
            Err(x) if x.kind() == io::ErrorKind::InvalidData => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_dir_should_return_success_if_removed_directory() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let _ = tempfile::tempfile_in(root.as_ref()).unwrap();

        match fsm.remove_dir(root.as_ref(), true).await {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn open_file_should_yield_error_if_underlying_open_fails() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let not_a_file = tempfile::tempdir_in(root.as_ref()).unwrap();

        match fsm.open_file(not_a_file.as_ref(), true, true, true).await {
            Err(x) if x.kind() == io::ErrorKind::Other => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn open_file_should_return_existing_open_file_if_permissions_allow() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        // Open with absolute path
        let handle = fsm
            .open_file(root.as_ref().join("test-file"), true, true, true)
            .await
            .expect("Failed to create file");

        assert_eq!(
            fsm.file_cnt(),
            1,
            "Unexpected number of open files: {}",
            fsm.file_cnt()
        );
        assert_eq!(
            fsm.get(handle).map(|f| f.permissions()),
            Some(LocalFilePermissions {
                read: true,
                write: true
            })
        );

        // Open with unresolved path (read-only)
        let handle_2 = fsm
            .open_file(
                root.as_ref().join(".").join("test-file"),
                false,
                false,
                true,
            )
            .await
            .expect("Failed to open file for read");

        assert_eq!(
            fsm.file_cnt(),
            1,
            "Unexpected number of open files: {}",
            fsm.file_cnt()
        );

        assert_eq!(handle, handle_2);

        assert_eq!(
            fsm.get(handle_2).map(|f| f.permissions()),
            Some(LocalFilePermissions {
                read: true,
                write: true
            })
        );

        // Open with absolute path (write-only)
        let handle_3 = fsm
            .open_file(root.as_ref().join("test-file"), false, true, false)
            .await
            .expect("Failed to open file for write");

        assert_eq!(
            fsm.file_cnt(),
            1,
            "Unexpected number of open files: {}",
            fsm.file_cnt()
        );

        assert_eq!(handle, handle_3);

        assert_eq!(
            fsm.get(handle_3).map(|f| f.permissions()),
            Some(LocalFilePermissions {
                read: true,
                write: true
            })
        );
    }

    #[tokio::test]
    async fn open_file_should_reopen_an_open_file_if_permissions_need_merging()
    {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        // Open write-only
        let handle = fsm
            .open_file(root.as_ref().join("test-file"), true, true, false)
            .await
            .expect("Failed to create file");

        assert_eq!(
            fsm.file_cnt(),
            1,
            "Unexpected number of open files: {}",
            fsm.file_cnt()
        );

        assert_eq!(
            fsm.get(handle).map(|f| f.permissions()),
            Some(LocalFilePermissions {
                read: false,
                write: true
            })
        );

        // Open read-only
        let handle_2 = fsm
            .open_file(root.as_ref().join("test-file"), false, false, true)
            .await
            .expect("Failed to open file");

        assert_eq!(
            fsm.file_cnt(),
            1,
            "Unexpected number of open files: {}",
            fsm.file_cnt()
        );

        assert_eq!(handle, handle_2);

        assert_eq!(
            fsm.get(handle_2).map(|f| f.permissions()),
            Some(LocalFilePermissions {
                read: true,
                write: true
            })
        );
    }

    #[tokio::test]
    async fn open_file_should_return_a_newly_opened_file_if_none_already_open()
    {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let handle = fsm
            .open_file(root.as_ref().join("test-file-1"), true, true, true)
            .await
            .expect("Failed to create file 1");

        let handle_2 = fsm
            .open_file(root.as_ref().join("test-file-2"), true, true, true)
            .await
            .expect("Failed to create file 2");

        assert_eq!(
            fsm.file_cnt(),
            2,
            "Unexpected number of open files: {}",
            fsm.file_cnt()
        );

        assert_ne!(handle, handle_2, "Two open files have same handle");
    }

    #[tokio::test]
    async fn close_file_should_yield_error_if_no_file_open_with_id() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let handle = fsm
            .open_file(root.as_ref().join("test-file"), true, true, true)
            .await
            .expect("Failed to create file");

        match fsm.close_file(LocalFileHandle {
            id: handle.id + 1,
            sig: handle.sig,
        }) {
            Err(x) if x.kind() == io::ErrorKind::NotFound => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn close_file_should_yield_error_if_file_has_different_signature() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let handle = fsm
            .open_file(root.as_ref().join("test-file"), true, true, true)
            .await
            .expect("Failed to create file");

        match fsm.close_file(LocalFileHandle {
            id: handle.id,
            sig: handle.sig + 1,
        }) {
            Err(x) if x.kind() == io::ErrorKind::InvalidInput => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn close_fould_should_remove_file_from_manager_if_successful() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let handle = fsm
            .open_file(root.as_ref().join("test-file"), true, true, true)
            .await
            .expect("Failed to create file");

        match fsm.close_file(handle) {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_file_should_yield_error_if_origin_path_does_not_exist() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let origin = root.as_ref().join("origin");
        let destination = root.as_ref().join("destination");

        match fsm.rename_file(origin, destination).await {
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::NotFound),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_file_should_yield_error_if_origin_path_is_not_a_file() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        // Make origin a directory instead of file
        let origin_dir = tempfile::tempdir_in(root.as_ref()).unwrap();

        let destination = root.as_ref().join("destination");

        match fsm.rename_file(origin_dir.as_ref(), destination).await {
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::Other),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_file_should_yield_error_if_file_is_open() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let origin = root.as_ref().join("file");
        let _file = fsm
            .open_file(origin.as_path(), true, true, true)
            .await
            .unwrap();

        let destination = root.as_ref().join("destination");

        match fsm
            .rename_file(origin.as_path(), destination.as_path())
            .await
        {
            Err(x) => assert_eq!(x.kind(), io::ErrorKind::InvalidData),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn rename_file_should_return_success_if_renamed_file() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let origin = tempfile::NamedTempFile::new_in(root.as_ref()).unwrap();

        let destination = root.as_ref().join("destination");

        match fsm.rename_file(origin, destination).await {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_file_should_yield_error_if_file_open() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let path = root.as_ref().join("test-file");

        fsm.open_file(path.as_path(), true, true, true)
            .await
            .expect("Failed to open file with manager");

        // Even though we want to remove everything, still cannot do it because
        // a local file is open
        match fsm.remove_file(path.as_path()).await {
            Err(x) if x.kind() == io::ErrorKind::InvalidData => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[tokio::test]
    async fn remove_file_should_return_success_if_removed_file() {
        let root = tempfile::tempdir().unwrap();
        let mut fsm = FileSystemManager::new();

        let file = tempfile::NamedTempFile::new_in(root.as_ref()).unwrap();

        match fsm.remove_file(file.as_ref()).await {
            Ok(_) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }
}
