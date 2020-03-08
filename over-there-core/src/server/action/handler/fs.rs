use crate::{
    msg::content::{
        io::{fs::*, IoErrorArgs},
        Content,
    },
    server::{
        action::ActionError,
        fs::{
            LocalDirEntry, LocalFileHandle, LocalFileReadError,
            LocalFileReadIoError, LocalFileWriteError, LocalFileWriteIoError,
        },
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

    // TODO: Check if file is already open based on canonical
    //       path, and return it if it is, otherwise open it
    //
    //       Also, should check that the open we are doing
    //       matches permissions for read/write access, otherwise
    //       we want to yield an error
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
            respond(Content::FileOpened(FileOpenedArgs { id, sig })).await
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

    let handle = LocalFileHandle {
        id: args.id,
        sig: args.sig,
    };

    match state.fs_manager.lock().await.close_file(handle).await {
        Ok(_) => respond(Content::FileClosed(FileClosedArgs {})).await,
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

    match state
        .fs_manager
        .lock()
        .await
        .rename_file(&args.from, &args.to)
        .await
    {
        Ok(_) => respond(Content::FileRenamed(FileRenamedArgs {})).await,
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

    match state.fs_manager.lock().await.remove_file(&args.path).await {
        Ok(_) => respond(Content::FileRemoved(FileRemovedArgs {})).await,
        Err(x) => respond(Content::IoError(From::from(x))).await,
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

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => match local_file.read_all(args.sig).await {
            Ok(data) => {
                respond(Content::FileContents(FileContentsArgs { data })).await
            }
            Err(LocalFileReadError::SigMismatch) => {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    sig: local_file.sig(),
                }))
                .await
            }
            Err(LocalFileReadError::IoError(x)) => {
                respond(Content::IoError(match x {
                    LocalFileReadIoError::SeekError(x) => {
                        IoErrorArgs::from_error_with_prefix(x, "Seek(0): ")
                    }
                    LocalFileReadIoError::ReadToEndError(x) => {
                        IoErrorArgs::from_error_with_prefix(x, "ReadToEnd: ")
                    }
                }))
                .await
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

    match state.fs_manager.lock().await.get_mut(args.id) {
        Some(local_file) => {
            match local_file.write_all(args.sig, &args.data).await {
                Ok(_) => {
                    respond(Content::FileWritten(FileWrittenArgs {
                        sig: local_file.sig(),
                    }))
                    .await
                }
                Err(LocalFileWriteError::SigMismatch) => {
                    respond(Content::FileSigChanged(FileSigChangedArgs {
                        sig: local_file.sig(),
                    }))
                    .await
                }
                Err(LocalFileWriteError::IoError(x)) => {
                    respond(Content::IoError(match x {
                        LocalFileWriteIoError::SeekError(x) => {
                            IoErrorArgs::from_error_with_prefix(x, "Seek(0): ")
                        }
                        LocalFileWriteIoError::SetLenError(x) => {
                            IoErrorArgs::from_error_with_prefix(
                                x,
                                "SetLen(0): ",
                            )
                        }
                        LocalFileWriteIoError::WriteAllError(x) => {
                            IoErrorArgs::from_error_with_prefix(x, "WriteAll: ")
                        }
                        LocalFileWriteIoError::FlushError(x) => {
                            IoErrorArgs::from_error_with_prefix(x, "Flush: ")
                        }
                    }))
                    .await
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
        Ok(_) => respond(Content::DirCreated(DirCreatedArgs {})).await,
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
        Ok(_) => respond(Content::DirRenamed(DirRenamedArgs {})).await,
        Err(x) => {
            let err: io::Error = x.into();
            respond(Content::IoError(From::from(err))).await
        }
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
        Ok(_) => respond(Content::DirRemoved(DirRemovedArgs {})).await,
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
                    respond(Content::DirContentsList(From::from(entries))).await
                }
                Err(x) => respond(Content::IoError(From::from(x))).await,
            }
        }
        Err(x) => {
            let e: io::Error = x.into();
            respond(Content::IoError(From::from(e))).await
        }
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
                path: tmp_path,
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
                path: tmp_file_path,
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
            Content::FileClosed(FileClosedArgs {}) => (),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_rename_file_should_send_error_if_file_open() {
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

        do_rename_file(
            Arc::clone(&state),
            &DoRenameFileArgs {
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
    async fn do_rename_file_should_send_confirmation_if_file_renamed() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();
        let from_path_str = file.as_ref().to_string_lossy().to_string();
        let to_path_str = format!("{}.renamed", from_path_str);

        do_rename_file(
            Arc::clone(&state),
            &DoRenameFileArgs {
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
            fs::metadata(from_path_str).await.is_err(),
            "File not renamed"
        );
        assert!(
            fs::metadata(to_path_str).await.is_ok(),
            "File renamed incorrectly"
        );

        match content.unwrap() {
            Content::FileRenamed(FileRenamedArgs {}) => (),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_remove_file_should_send_error_if_file_open() {
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

        do_remove_file(
            Arc::clone(&state),
            &DoRemoveFileArgs {
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
    async fn do_remove_file_should_send_confirmation_if_file_removed() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;

        let file = tempfile::NamedTempFile::new().unwrap();

        do_remove_file(
            Arc::clone(&state),
            &DoRemoveFileArgs {
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
            Content::FileRemoved(FileRemovedArgs {}) => (),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_read_file_should_send_contents_if_read_successful() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let file_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let mut file = tempfile::NamedTempFile::new().unwrap();

        use std::io::Write;
        file.write_all(&file_data).unwrap();
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
            Content::FileContents(FileContentsArgs { data }) => {
                assert_eq!(data, file_data);
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
            Content::FileSigChanged(FileSigChangedArgs { sig: cur_sig }) => {
                assert_eq!(cur_sig, sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_success_if_write_successful() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

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
                data: data.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileWritten(FileWrittenArgs { sig: new_sig }) => {
                assert_ne!(new_sig, sig);

                use std::io::{Seek, SeekFrom};
                file.seek(SeekFrom::Start(0)).unwrap();

                use std::io::Read;
                let mut file_data = Vec::new();
                file.read_to_end(&mut file_data).unwrap();

                assert_eq!(
                    data, file_data,
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
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

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
            &DoWriteFileArgs { id, sig, data },
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
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

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
                data: data.clone(),
            },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs { sig: cur_sig }) => {
                assert_eq!(cur_sig, sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_create_dir_should_send_error_if_directory_outside_root() {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::new(root_path.as_ref()));
        let mut content: Option<Content> = None;

        let dir_path = tempfile::tempdir()
            .unwrap()
            .as_ref()
            .to_string_lossy()
            .to_string();

        do_create_dir(
            Arc::clone(&state),
            &DoCreateDirArgs {
                path: dir_path.clone(),
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
            fs::metadata(&dir_path).await.is_err(),
            "Dir was unexpectedly created"
        );

        match content.unwrap() {
            Content::IoError(IoErrorArgs { error_kind, .. }) => {
                assert_eq!(error_kind, io::ErrorKind::PermissionDenied);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_create_dir_should_send_error_if_part_of_path_missing_and_flag_not_set(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::new(root_path.as_ref()));
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
        let state = Arc::new(ServerState::new(root_path.as_ref()));
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
            Content::DirCreated(DirCreatedArgs {}) => (),
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_create_dir_should_send_confirmation_if_multi_level_directory_created(
    ) {
        let root_path = tempfile::tempdir().unwrap();
        let state = Arc::new(ServerState::new(root_path.as_ref()));
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
            Content::DirCreated(DirCreatedArgs {}) => (),
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
            Content::DirRenamed(DirRenamedArgs {}) => (),
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
            Content::DirRemoved(DirRemovedArgs {}) => (),
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
            Content::DirRemoved(DirRemovedArgs {}) => (),
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