mod dir;
mod file;

pub use dir::{LocalDirEntriesError, LocalDirEntry, LocalDirRenameError};

pub use file::{
    LocalFile, LocalFileHandle, LocalFileReadError, LocalFileReadIoError,
    LocalFileRemoveError, LocalFileRenameError, LocalFileWriteError,
    LocalFileWriteIoError,
};

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug)]
struct LocalFilePermissions {
    write: bool,
    read: bool,
}

#[derive(Debug)]
pub struct FileSystemManager {
    root: Option<PathBuf>,
    files: HashMap<u32, (LocalFile, LocalFilePermissions)>,
}

impl Default for FileSystemManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemManager {
    /// Creates new instance where operations are allowed anywhere
    pub fn new() -> Self {
        Self {
            root: None,
            files: HashMap::new(),
        }
    }

    /// Creates new instance where operations are only allowed within
    /// the current directory as defined by `std::env::current_dir`
    pub fn with_current_dir() -> io::Result<Self> {
        Ok(Self::with_root(std::env::current_dir()?))
    }

    /// Creates new instance where operations are only allowed within `root`
    pub fn with_root(root: impl AsRef<Path>) -> Self {
        Self {
            root: Some(root.as_ref().to_path_buf()),
            files: HashMap::new(),
        }
    }

    /// Creates a new directory
    pub async fn create_dir(
        &self,
        path: impl AsRef<Path>,
        create_components: bool,
    ) -> io::Result<()> {
        let path = self.validate_path(path.as_ref()).await?;
        dir::create(path, create_components).await
    }

    /// Attempts to rename an entire directory, checking through all open
    /// files to see if their paths also need to be renamed.
    ///
    /// If any file is open, its sig will need to be included in the auth
    /// section to ensure that it can be moved; otherwise, an error will
    /// be returned
    pub async fn rename_dir(
        &mut self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> Result<(), LocalDirRenameError> {
        let from = self
            .validate_path(from.as_ref())
            .await
            .map_err(LocalDirRenameError::IoError)?;
        let to = self
            .validate_path(to.as_ref())
            .await
            .map_err(LocalDirRenameError::IoError)?;

        dir::rename(from.as_path(), to.as_path()).await?;

        // TODO: Perform more optimal renames by filtering down open files
        //       using a path tree?
        for f in self.files.values_mut() {
            f.0.apply_path_changed(from.as_path(), to.as_path());
        }

        Ok(())
    }

    /// Attempts to remove an entire directory, failing if any file is
    /// currently open within the directory.
    pub async fn remove_dir(
        &mut self,
        path: impl AsRef<Path>,
        non_empty: bool,
    ) -> io::Result<()> {
        let path = self.validate_path(path.as_ref()).await?;

        // TODO: Perform more optimal removal check by filtering down open
        //       files using a path tree?
        for f in self.files.values() {
            if f.0.path().starts_with(path.as_path()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Directory contains open files",
                ));
            }
        }

        // No open file is within this directory, so good to attempt to remove
        dir::remove(path, non_empty).await
    }

    /// Retrieves all entries within the directory `path`.
    ///
    /// This is a non-recursive operation, meaning that it will only yield
    /// the immediate directory entires and not walk through subdirectories
    /// or follow symlinks.
    ///
    /// Will yield an error if the path is not within the specified `root`,
    /// or if there are complications with reading the directory and its
    /// entries.
    pub async fn dir_entries(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<LocalDirEntry>, LocalDirEntriesError> {
        let path = self
            .validate_path(path.as_ref())
            .await
            .map_err(LocalDirEntriesError::ReadDirError)?;

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
        let path = self.validate_path(path.as_ref()).await?;

        let mut new_permissions = LocalFilePermissions { read, write };
        let mut maybe_id_and_sig = None;

        // TODO: Perform more optimal lookup by filtering down open files
        //       using a path tree?
        let search = self
            .files
            .values_mut()
            .find(|f| f.0.path() == path.as_path());

        // If we found a match, check the permissions to see if we can return
        // it or if we need to open a new copy with the proper merged
        // permissions
        if let Some((file, permissions)) = search {
            let id = file.id();
            let sig = file.sig();

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
        self.files
            .insert(new_file.id(), (new_file, new_permissions));

        Ok(handle)
    }

    /// Closes an open file by `id`, returning whether or not there was a file
    /// to close with the specified `id`
    pub async fn close_file(&mut self, id: u32) -> bool {
        self.files.remove(&id).is_some()
    }

    /// Adds the already-created `local_file`, specifying what permissions
    /// it current has for `write` and `read`.
    ///
    /// NOTE: These permissions MUST match what was specified when opening
    ///       the local file.
    ///
    /// TODO: This is a method only used for testing and should be removed
    ///       when we clean up this interface.
    pub(crate) fn add_existing_file(
        &mut self,
        local_file: LocalFile,
        write: bool,
        read: bool,
    ) -> LocalFileHandle {
        let handle = local_file.handle();
        let permissions = LocalFilePermissions { read, write };
        self.files.insert(handle.id, (local_file, permissions));
        handle
    }

    /// Looks up an open file by its associated `id`
    pub fn get_mut(&mut self, id: u32) -> Option<&mut LocalFile> {
        match self.files.get_mut(&id) {
            Some((file, _)) => Some(file),
            None => None,
        }
    }

    /// Looks up an open file by its associated `id`
    pub fn get(&self, id: u32) -> Option<&LocalFile> {
        match self.files.get(&id) {
            Some((file, _)) => Some(file),
            None => None,
        }
    }

    /// Checks `path` to see if is okay, returning the fully-realized path.
    ///
    /// If `path` is relative, it is placed within the root of the manager.
    /// If `path` is absolute, check if `path` is in `root`
    /// 1. If so, then this function returns ok with the `path`
    /// 3. Otherwise we have a bad path and this function returns an error
    async fn validate_path(
        &self,
        path: impl AsRef<Path>,
    ) -> io::Result<PathBuf> {
        let is_ok = self
            .root
            .as_ref()
            .map(|r| path.as_ref().starts_with(r))
            .unwrap_or(true);

        if is_ok {
            fs::canonicalize(path).await
        } else {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_dir_should_yield_error_if_path_not_in_root() {
        let fsm = FileSystemManager::with_root("/some/path");

        let result = fsm.create_dir("/some/dir", true).await;
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }

    #[tokio::test]
    async fn create_dir_should_yield_error_if_parent_dirs_missing_and_flag_not_set(
    ) {
        let fsm = FileSystemManager::new();

        let result = fsm.create_dir("some/dir", false).await;
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn create_dir_should_return_success_if_created_the_path() {
        unimplemented!();
    }

    #[tokio::test]
    async fn rename_dir_should_yield_error_if_origin_path_not_in_root() {
        unimplemented!();
    }

    #[tokio::test]
    async fn rename_dir_should_yield_error_if_destination_path_not_in_root() {
        unimplemented!();
    }

    #[tokio::test]
    async fn rename_dir_should_yield_error_if_origin_path_is_not_a_directory() {
        unimplemented!();
    }

    #[tokio::test]
    async fn rename_dir_should_yield_error_if_unable_to_rename_directory() {
        unimplemented!();
    }

    #[tokio::test]
    async fn rename_dir_should_update_paths_of_open_files_in_directory() {
        unimplemented!();
    }

    #[tokio::test]
    async fn rename_dir_should_return_success_if_renamed_directory() {
        unimplemented!();
    }

    #[tokio::test]
    async fn dir_entries_should_yield_error_if_path_not_in_root() {
        unimplemented!();
    }

    #[tokio::test]
    async fn dir_entries_should_yield_error_if_path_not_a_file() {
        unimplemented!();
    }

    #[tokio::test]
    async fn dir_entries_should_return_a_list_of_immediate_entries_in_a_directory(
    ) {
        unimplemented!();
    }

    #[tokio::test]
    async fn remove_dir_should_yield_error_if_path_not_in_root() {
        unimplemented!();
    }

    #[tokio::test]
    async fn remove_dir_should_yield_error_if_directory_not_empty_and_flag_not_set(
    ) {
        unimplemented!();
    }

    #[tokio::test]
    async fn remove_dir_should_yield_error_if_open_files_exist_in_directory() {
        unimplemented!();
    }

    #[tokio::test]
    async fn remove_dir_should_return_success_if_removed_directory() {
        unimplemented!();
    }

    #[tokio::test]
    async fn open_file_should_yield_error_if_path_not_in_root() {
        unimplemented!();
    }

    #[tokio::test]
    async fn open_file_should_yield_error_if_underlying_open_fails() {
        unimplemented!();
    }

    #[tokio::test]
    async fn open_file_should_return_existing_open_file_if_permissions_allow() {
        unimplemented!();
    }

    #[tokio::test]
    async fn open_file_should_reopen_an_open_file_if_permissions_need_merging()
    {
        unimplemented!();
    }

    #[tokio::test]
    async fn open_file_should_return_a_newly_opened_file_if_none_already_open()
    {
        unimplemented!();
    }
}
