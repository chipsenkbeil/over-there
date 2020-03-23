use crate::{
    msg::content::*,
    server::{
        action::ActionError,
        fs::{LocalDirEntry, LocalFileError, LocalFileHandle},
        state::ServerState,
    },
};
use log::debug;
use std::convert::TryFrom;
use std::future::Future;
use std::io;
use std::sync::Arc;

pub async fn do_open_file<F, R>(
    state: Arc<ServerState>,
    args: &DoOpenFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_open_file: {:?}", args);

    match state
        .fs_manager
        .lock()
        .await
        .open_file(
            &args.path,
            args.create_if_missing,
            args.write_access,
            args.read_access,
        )
        .await
    {
        Ok(LocalFileHandle { id, sig }) => {
            state.touch_file_id(id).await;
            respond(Content::FileOpened(FileOpenedArgs {
                id,
                sig,
                path: args.path.clone(),
                read: args.read_access,
                write: args.write_access,
            }))
            .await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_close_file<F, R>(
    state: Arc<ServerState>,
    args: &DoCloseFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_close_file: {:?}", args);
    state.touch_file_id(args.id).await;

    let handle = LocalFileHandle {
        id: args.id,
        sig: args.sig,
    };

    match state.fs_manager.lock().await.close_file(handle) {
        Ok(_) => {
            state.remove_file_id(args.id).await;
            respond(Content::FileClosed(FileClosedArgs { id: args.id })).await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_rename_unopened_file<F, R>(
    state: Arc<ServerState>,
    args: &DoRenameUnopenedFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_rename_unopened_file: {:?}", args);

    match state
        .fs_manager
        .lock()
        .await
        .rename_file(&args.from, &args.to)
        .await
    {
        Ok(_) => {
            respond(Content::UnopenedFileRenamed(UnopenedFileRenamedArgs {
                from: args.from.clone(),
                to: args.to.clone(),
            }))
            .await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_rename_file<F, R>(
    state: Arc<ServerState>,
    args: &DoRenameFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_rename_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => match local_file.rename(args.sig, &args.to).await {
            Ok(_) => {
                respond(Content::FileRenamed(FileRenamedArgs {
                    id: args.id,
                    sig: local_file.sig(),
                }))
                .await
            }
            Err(LocalFileError::SigMismatch) => {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    id: args.id,
                    sig: local_file.sig(),
                }))
                .await
            }
            Err(LocalFileError::IoError(x)) => {
                respond(Content::IoError(From::from(x))).await
            }
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_file_id(args.id)))
                .await
        }
    }
}

pub async fn do_remove_unopened_file<F, R>(
    state: Arc<ServerState>,
    args: &DoRemoveUnopenedFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_remove_unopened_file: {:?}", args);

    match state.fs_manager.lock().await.remove_file(&args.path).await {
        Ok(_) => {
            respond(Content::UnopenedFileRemoved(UnopenedFileRemovedArgs {
                path: args.path.clone(),
            }))
            .await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_remove_file<F, R>(
    state: Arc<ServerState>,
    args: &DoRemoveFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_remove_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => match local_file.remove(args.sig).await {
            Ok(_) => {
                state.remove_file_id(args.id).await;
                respond(Content::FileRemoved(FileRemovedArgs {
                    id: args.id,
                    sig: local_file.sig(),
                }))
                .await
            }
            Err(LocalFileError::SigMismatch) => {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    id: args.id,
                    sig: local_file.sig(),
                }))
                .await
            }
            Err(LocalFileError::IoError(x)) => {
                respond(Content::IoError(From::from(x))).await
            }
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_file_id(args.id)))
                .await
        }
    }
}

pub async fn do_read_file<F, R>(
    state: Arc<ServerState>,
    args: &DoReadFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_read_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => match local_file.read_all(args.sig).await {
            Ok(contents) => {
                respond(Content::FileContents(FileContentsArgs {
                    id: args.id,
                    contents,
                }))
                .await
            }
            Err(LocalFileError::SigMismatch) => {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    id: args.id,
                    sig: local_file.sig(),
                }))
                .await
            }
            Err(LocalFileError::IoError(x)) => {
                respond(Content::IoError(From::from(x))).await
            }
        },
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_file_id(args.id)))
                .await
        }
    }
}

pub async fn do_write_file<F, R>(
    state: Arc<ServerState>,
    args: &DoWriteFileArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_write_file: {:?}", args);
    state.touch_file_id(args.id).await;

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => {
            match local_file.write_all(args.sig, &args.contents).await {
                Ok(_) => {
                    respond(Content::FileWritten(FileWrittenArgs {
                        id: args.id,
                        sig: local_file.sig(),
                    }))
                    .await
                }
                Err(LocalFileError::SigMismatch) => {
                    respond(Content::FileSigChanged(FileSigChangedArgs {
                        id: args.id,
                        sig: local_file.sig(),
                    }))
                    .await
                }
                Err(LocalFileError::IoError(x)) => {
                    respond(Content::IoError(From::from(x))).await
                }
            }
        }
        None => {
            respond(Content::IoError(IoErrorArgs::invalid_file_id(args.id)))
                .await
        }
    }
}

pub async fn do_create_dir<F, R>(
    state: Arc<ServerState>,
    args: &DoCreateDirArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_create_dir: {:?}", args);

    match state
        .fs_manager
        .lock()
        .await
        .create_dir(&args.path, args.include_components)
        .await
    {
        Ok(_) => {
            respond(Content::DirCreated(DirCreatedArgs {
                path: args.path.clone(),
            }))
            .await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_rename_dir<F, R>(
    state: Arc<ServerState>,
    args: &DoRenameDirArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_rename_dir: {:?}", args);

    match state
        .fs_manager
        .lock()
        .await
        .rename_dir(&args.from, &args.to)
        .await
    {
        Ok(_) => {
            respond(Content::DirRenamed(DirRenamedArgs {
                from: args.from.clone(),
                to: args.to.clone(),
            }))
            .await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_remove_dir<F, R>(
    state: Arc<ServerState>,
    args: &DoRemoveDirArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_remove_dir: {:?}", args);

    match state
        .fs_manager
        .lock()
        .await
        .remove_dir(&args.path, args.non_empty)
        .await
    {
        Ok(_) => {
            respond(Content::DirRemoved(DirRemovedArgs {
                path: args.path.clone(),
            }))
            .await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

pub async fn do_list_dir_contents<F, R>(
    state: Arc<ServerState>,
    args: &DoListDirContentsArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_list_dir_contents: {:?}", args);

    match state.fs_manager.lock().await.dir_entries(&args.path).await {
        Ok(local_entries) => {
            let entries: io::Result<Vec<DirEntry>> =
                local_entries.into_iter().map(DirEntry::try_from).collect();
            match entries {
                Ok(entries) => {
                    respond(Content::DirContentsList(DirContentsListArgs {
                        path: args.path.clone(),
                        entries,
                    }))
                    .await
                }
                Err(x) => respond(Content::IoError(From::from(x))).await,
            }
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

impl TryFrom<LocalDirEntry> for DirEntry {
    type Error = io::Error;

    fn try_from(local_dir_entry: LocalDirEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            path: local_dir_entry
                .path
                .into_os_string()
                .into_string()
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "OS String does not contain valid unicode",
                    )
                })?,
            is_file: local_dir_entry.is_file,
            is_dir: local_dir_entry.is_dir,
            is_symlink: local_dir_entry.is_symlink,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use tokio::fs;

    #[tokio::test]
    async fn do_open_file_should_send_success_if_create_flag_set_and_opening_new_file(
    ) {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let tmp_path = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        do_open_file(
            Arc::clone(&state),
            &DoOpenFileArgs {
                path: tmp_path.clone(),
                create_if_missing: true,
                write_access: true,
                read_access: true,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileOpened(args) => {
                let x = state.fs_manager.lock().await;
                let local_file = x.get(args.id).unwrap();
                assert_eq!(args.sig, local_file.sig());
                assert_eq!(args.path, tmp_path);
                assert!(args.write);
                assert!(args.read);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_open_file_should_send_success_opening_existing_file() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        let tmp_file_path = tmp_file.path().to_string_lossy().to_string();

        do_open_file(
            Arc::clone(&state),
            &DoOpenFileArgs {
                path: tmp_file_path.clone(),
                create_if_missing: false,
                write_access: true,
                read_access: true,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileOpened(args) => {
                let x = state.fs_manager.lock().await;
                let local_file = x.get(args.id).unwrap();
                assert_eq!(args.sig, local_file.sig());
                assert_eq!(args.path, tmp_file_path);
                assert!(args.write);
                assert!(args.read);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_open_file_should_send_error_if_file_missing_and_create_flag_not_set(
    ) {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let tmp_path = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        do_open_file(
            Arc::clone(&state),
            &DoOpenFileArgs {
                path: tmp_path,
                create_if_missing: false,
                write_access: true,
                read_access: true,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_close_file_should_send_error_if_file_not_open() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let tmp_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_path, false, false, true)
            .await
            .expect("Failed to open file");

        let id = handle.id + 1;
        let sig = handle.sig;

        do_close_file(Arc::clone(&state), &DoCloseFileArgs { id, sig }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_close_file_should_send_error_if_signature_different() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let tmp_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_path, false, false, true)
            .await
            .expect("Failed to open file");

        let id = handle.id;
        let sig = handle.sig + 1;

        do_close_file(Arc::clone(&state), &DoCloseFileArgs { id, sig }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_close_file_should_send_confirmation_if_successful() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let tmp_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_path, false, false, true)
            .await
            .expect("Failed to open file");

        let id = handle.id;
        let sig = handle.sig;

        do_close_file(Arc::clone(&state), &DoCloseFileArgs { id, sig }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileClosed(FileClosedArgs { id: arg_id }) => {
                assert_eq!(arg_id, id)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_unopened_file_should_send_error_if_file_open() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        do_rename_unopened_file(
            Arc::clone(&state),
            &DoRenameUnopenedFileArgs {
                from: file.as_ref().to_string_lossy().to_string(),
                to: file.as_ref().to_string_lossy().to_string(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidData)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_unopened_file_should_send_confirmation_if_file_renamed()
    {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let from_path_str = file.as_ref().to_string_lossy().to_string();
        let to_path_str = format!("{}.renamed", from_path_str);

        do_rename_unopened_file(
            Arc::clone(&state),
            &DoRenameUnopenedFileArgs {
                from: from_path_str.clone(),
                to: to_path_str.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(from_path_str.clone()).await.is_err(),
            "File not renamed"
        );
        assert!(
            fs::metadata(to_path_str.clone()).await.is_ok(),
            "File renamed incorrectly"
        );

        match content.unwrap() {
            Content::UnopenedFileRenamed(UnopenedFileRenamedArgs {
                from,
                to,
            }) => {
                assert_eq!(from, from_path_str);
                assert_eq!(to, to_path_str);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_file_should_send_error_if_file_not_open() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");
        let new_path_str = String::from("new-file-name");

        do_rename_file(
            Arc::clone(&state),
            &DoRenameFileArgs {
                id: handle.id + 1,
                sig: handle.sig,
                to: new_path_str.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File missing when rename failed"
        );

        assert!(
            fs::metadata(&new_path_str).await.is_err(),
            "Renamed file exists"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_file_should_send_error_if_signature_different() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");
        let new_path_str = String::from("new-file-name");

        do_rename_file(
            Arc::clone(&state),
            &DoRenameFileArgs {
                id: handle.id,
                sig: handle.sig + 1,
                to: new_path_str.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File missing when rename failed"
        );

        assert!(
            fs::metadata(&new_path_str).await.is_err(),
            "Renamed file exists"
        );

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs { id, sig }) => {
                assert_eq!(id, handle.id);
                assert_eq!(sig, handle.sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_file_should_send_confirmation_if_file_renamed() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");
        let new_path_str =
            format!("{}.2", file.as_ref().to_string_lossy().to_string());

        do_rename_file(
            Arc::clone(&state),
            &DoRenameFileArgs {
                id: handle.id,
                sig: handle.sig,
                to: new_path_str.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "Renamed file still exists at original location"
        );

        assert!(
            fs::metadata(&new_path_str).await.is_ok(),
            "Renamed file missing"
        );

        match content.unwrap() {
            Content::FileRenamed(FileRenamedArgs { id, sig }) => {
                assert_eq!(handle.id, id, "Wrong id returned");
                assert_ne!(
                    handle.sig, sig,
                    "Signature returned is not different"
                );
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_unopened_file_should_send_error_if_file_open() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        do_remove_unopened_file(
            Arc::clone(&state),
            &DoRemoveUnopenedFileArgs {
                path: file.as_ref().to_string_lossy().to_string(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File unexpectedly removed"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidData)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_unopened_file_should_send_confirmation_if_file_removed()
    {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();

        do_remove_unopened_file(
            Arc::clone(&state),
            &DoRemoveUnopenedFileArgs {
                path: file.as_ref().to_string_lossy().to_string(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "File still exists"
        );

        match content.unwrap() {
            Content::UnopenedFileRemoved(UnopenedFileRemovedArgs { path }) => {
                assert_eq!(path, file.as_ref().to_string_lossy().to_string());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_file_should_send_error_if_file_not_open() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        do_remove_file(
            Arc::clone(&state),
            &DoRemoveFileArgs {
                id: handle.id + 1,
                sig: handle.sig,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File unexpectedly missing after failed remove"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_file_should_send_error_if_signature_different() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        do_remove_file(
            Arc::clone(&state),
            &DoRemoveFileArgs {
                id: handle.id,
                sig: handle.sig + 1,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File unexpectedly missing after failed remove"
        );

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs { id, sig }) => {
                assert_eq!(id, handle.id);
                assert_eq!(sig, handle.sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_file_should_send_confirmation_if_file_removed() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Failed to open file");

        do_remove_file(
            Arc::clone(&state),
            &DoRemoveFileArgs {
                id: handle.id,
                sig: handle.sig,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "File still exists"
        );

        match content.unwrap() {
            Content::FileRemoved(FileRemovedArgs { id, sig }) => {
                assert_eq!(handle.id, id, "Wrong id returned");
                assert_ne!(
                    handle.sig, sig,
                    "Signature returned is not different"
                );
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_contents_if_read_successful() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let file_contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let mut file = tempfile::NamedTempFile::new().unwrap();

        use std::io::Write;
        file.write_all(&file_contents).unwrap();
        file.flush().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        do_read_file(Arc::clone(&state), &DoReadFileArgs { id, sig }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileContents(FileContentsArgs {
                id: arg_id,
                contents,
            }) => {
                assert_eq!(id, arg_id, "Wrong id returned");
                assert_eq!(contents, file_contents);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_error_if_file_not_open() {
        let mut content: Option<Content> = None;

        do_read_file(
            Arc::new(ServerState::default()),
            &DoReadFileArgs { id: 0, sig: 0 },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidInput);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_error_if_not_readable() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let tmp_file = tempfile::NamedTempFile::new().unwrap();

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .read(false)
            .write(true)
            .create(true)
            .open(tmp_file.path())
            .unwrap();
        file.write_all(&vec![1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
        file.flush().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(tmp_file.as_ref(), true, true, false)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        do_read_file(Arc::clone(&state), &DoReadFileArgs { id, sig }, |c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { os_code, .. }) => {
                // Should be an OS-related error
                assert!(os_code.is_some());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_error_if_file_sig_has_changed() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        do_read_file(
            Arc::clone(&state),
            &DoReadFileArgs { id, sig: sig + 1 },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs {
                id: cur_id,
                sig: cur_sig,
            }) => {
                assert_eq!(cur_id, id);
                assert_eq!(cur_sig, sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_success_if_write_successful() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let mut file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        do_write_file(
            Arc::clone(&state),
            &DoWriteFileArgs {
                id,
                sig,
                contents: contents.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileWritten(FileWrittenArgs {
                id: cur_id,
                sig: new_sig,
            }) => {
                assert_eq!(cur_id, id, "Wrong id returned");
                assert_ne!(new_sig, sig);

                use std::io::{Seek, SeekFrom};
                file.seek(SeekFrom::Start(0)).unwrap();

                use std::io::Read;
                let mut file_contents = Vec::new();
                file.read_to_end(&mut file_contents).unwrap();

                assert_eq!(
                    contents, file_contents,
                    "File does not match written content"
                );
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_error_if_not_writeable() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), false, false, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        do_write_file(
            Arc::clone(&state),
            &DoWriteFileArgs { id, sig, contents },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { os_code, .. }) => {
                // Should be an OS-related error
                assert!(os_code.is_some());
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_error_if_file_sig_has_changed() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let contents = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let file = tempfile::NamedTempFile::new().unwrap();

        let handle = state
            .fs_manager
            .lock()
            .await
            .open_file(file.as_ref(), true, true, true)
            .await
            .expect("Unable to open file");
        let id = handle.id;
        let sig = handle.sig;

        do_write_file(
            Arc::clone(&state),
            &DoWriteFileArgs {
                id,
                sig: sig + 1,
                contents: contents.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs {
                id: cur_id,
                sig: cur_sig,
            }) => {
                assert_eq!(cur_id, id, "Wrong id returned");
                assert_eq!(cur_sig, sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_create_dir_should_send_error_if_part_of_path_missing_and_flag_not_set(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let dir_path = root_path.as_ref().join("test").join("dir");

        do_create_dir(
            Arc::clone(&state),
            &DoCreateDirArgs {
                path: dir_path.as_path().to_string_lossy().to_string(),
                include_components: false,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(&dir_path).await.is_err(),
            "Dir was unexpectedly created"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_create_dir_should_send_confirmation_if_single_level_directory_created(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let dir_path = root_path.as_ref().join("test");

        do_create_dir(
            Arc::clone(&state),
            &DoCreateDirArgs {
                path: dir_path.as_path().to_string_lossy().to_string(),
                include_components: false,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(&dir_path).await.is_ok(),
            "Dir was unexpectedly not created"
        );

        match content.unwrap() {
            Content::DirCreated(DirCreatedArgs { path }) => {
                assert_eq!(
                    path,
                    dir_path.to_string_lossy().to_string(),
                    "Wrong path returned"
                );
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_create_dir_should_send_confirmation_if_multi_level_directory_created(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let dir_path = root_path.as_ref().join("test").join("dir");

        do_create_dir(
            Arc::clone(&state),
            &DoCreateDirArgs {
                path: dir_path.as_path().to_string_lossy().to_string(),
                include_components: true,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(&dir_path).await.is_ok(),
            "Dir was unexpectedly not created"
        );

        match content.unwrap() {
            Content::DirCreated(DirCreatedArgs { path }) => assert_eq!(
                path,
                dir_path.to_string_lossy().to_string(),
                "Wrong path returned"
            ),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_dir_should_send_error_if_a_file_open_in_directory() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap().into_path();
        fs::remove_dir(to_dir.as_path())
            .await
            .expect("Failed to clean up temp dir");

        let file_path = from_dir.as_ref().join("test-file");
        let _ = state
            .fs_manager
            .lock()
            .await
            .open_file(file_path.as_path(), true, true, true)
            .await
            .expect("Failed to open file");

        do_rename_dir(
            Arc::clone(&state),
            &DoRenameDirArgs {
                from: from_dir.as_ref().to_string_lossy().to_string(),
                to: to_dir.as_path().to_string_lossy().to_string(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file_path.as_path()).await.is_ok(),
            "Open file unexpectedly renamed to something else"
        );

        assert!(
            fs::metadata(from_dir.as_ref()).await.is_ok(),
            "Dir was unexpectedly renamed"
        );

        assert!(
            fs::metadata(to_dir.as_path()).await.is_err(),
            "Destination of rename unexpectedly exists"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidData);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_dir_should_send_confirmation_if_directory_renamed() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap().into_path();
        fs::remove_dir(to_dir.as_path())
            .await
            .expect("Failed to clean up temp dir");

        do_rename_dir(
            Arc::clone(&state),
            &DoRenameDirArgs {
                from: from_dir.as_ref().to_string_lossy().to_string(),
                to: to_dir.as_path().to_string_lossy().to_string(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(from_dir.as_ref()).await.is_err(),
            "Dir was unexpectedly not renamed"
        );

        assert!(
            fs::metadata(to_dir.as_path()).await.is_ok(),
            "Destination of rename unexpectedly missing"
        );

        match content.unwrap() {
            Content::DirRenamed(DirRenamedArgs { from, to }) => {
                assert_eq!(
                    from,
                    from_dir.as_ref().to_string_lossy().to_string(),
                    "Wrong from path returned"
                );
                assert_eq!(
                    to,
                    to_dir.to_string_lossy().to_string(),
                    "Wrong to path returned"
                );
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_dir_should_send_error_if_a_file_open_in_directory() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let dir = tempfile::tempdir().unwrap();

        let file_path = dir.as_ref().join("test-file");
        let _ = state
            .fs_manager
            .lock()
            .await
            .open_file(file_path.as_path(), true, true, true)
            .await
            .expect("Failed to open file");

        do_remove_dir(
            Arc::clone(&state),
            &DoRemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: true,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file_path.as_path()).await.is_ok(),
            "Open file unexpectedly removed"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::InvalidData);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_dir_should_send_confirmation_if_empty_directory_removed()
    {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let dir = tempfile::tempdir().unwrap();

        do_remove_dir(
            Arc::clone(&state),
            &DoRemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: false,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(dir.as_ref()).await.is_err(),
            "Dir still exists"
        );

        match content.unwrap() {
            Content::DirRemoved(DirRemovedArgs { path }) => assert_eq!(
                path,
                dir.as_ref().to_string_lossy().to_string(),
                "Wrong path returned"
            ),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_dir_should_send_error_if_nonempty_directory_removed_with_flag_not_set(
    ) {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let dir = tempfile::tempdir().unwrap();
        let file = tempfile::NamedTempFile::new_in(dir.as_ref()).unwrap();

        do_remove_dir(
            Arc::clone(&state),
            &DoRemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: false,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_ok(),
            "File in dir unexpectedly removed"
        );

        assert!(
            fs::metadata(dir.as_ref()).await.is_ok(),
            "Dir unexpected removed"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::Other);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_dir_should_send_confirmation_if_nonempty_directory_removed_with_flag_set(
    ) {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let dir = tempfile::tempdir().unwrap();
        let file = tempfile::NamedTempFile::new_in(dir.as_ref()).unwrap();

        do_remove_dir(
            Arc::clone(&state),
            &DoRemoveDirArgs {
                path: dir.as_ref().to_string_lossy().to_string(),
                non_empty: true,
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(
            fs::metadata(file.as_ref()).await.is_err(),
            "File in dir still exists"
        );

        assert!(
            fs::metadata(dir.as_ref()).await.is_err(),
            "Dir still exists"
        );

        match content.unwrap() {
            Content::DirRemoved(DirRemovedArgs { path }) => assert_eq!(
                path,
                dir.as_ref().to_string_lossy().to_string(),
                "Wrong path returned"
            ),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_list_dir_contents_should_send_entries_if_successful() {
        let mut content: Option<Content> = None;

        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_string_lossy().to_string();

        let tmp_file = tempfile::NamedTempFile::new_in(&dir).unwrap();
        let tmp_dir = tempfile::tempdir_in(&dir).unwrap();

        do_list_dir_contents(
            Arc::new(ServerState::default()),
            &DoListDirContentsArgs {
                path: dir_path.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        std::fs::remove_dir_all(dir_path).unwrap();

        match content.unwrap() {
            Content::DirContentsList(args) => {
                assert_eq!(
                    args.entries.len(),
                    2,
                    "Unexpected number of entries"
                );

                assert!(args.entries.contains(&DirEntry {
                    path: tmp_file.path().to_string_lossy().to_string(),
                    is_file: true,
                    is_dir: false,
                    is_symlink: false
                }));

                assert!(args.entries.contains(&DirEntry {
                    path: tmp_dir.path().to_string_lossy().to_string(),
                    is_file: false,
                    is_dir: true,
                    is_symlink: false
                }));
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_list_dir_contents_should_send_error_if_path_invalid() {
        let mut content: Option<Content> = None;

        do_list_dir_contents(
            Arc::new(ServerState::default()),
            &DoListDirContentsArgs {
                path: String::from(""),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::NotFound)
            }
            x => panic!("Bad content: {:?}", x),
        }
    }
}
