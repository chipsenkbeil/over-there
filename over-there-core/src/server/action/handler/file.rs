use crate::{
    msg::content::{
        io::{file::*, IoErrorArgs},
        Content,
    },
    server::{
        action::ActionError,
        file::{
            LocalFile, LocalFileReadError, LocalFileReadIoError,
            LocalFileWriteError, LocalFileWriteIoError,
        },
        state::ServerState,
    },
};
use log::debug;
use std::future::Future;
use std::io;
use std::sync::Arc;
use tokio::fs;

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
    match LocalFile::open(
        &args.path,
        args.create_if_missing,
        args.write_access,
        args.read_access,
    )
    .await
    {
        Ok(local_file) => {
            let id = local_file.id();
            let sig = local_file.sig();

            // Store the opened file so we can operate on it later
            state.files.lock().await.insert(id, local_file);
            respond(Content::FileOpened(FileOpenedArgs { id, sig })).await
        }
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

    match state.files.lock().await.get_mut(&args.id) {
        Some(local_file) => match local_file.read_all(args.sig).await {
            Ok(data) => {
                respond(Content::FileContents(FileContentsArgs { data })).await
            }
            Err(LocalFileReadError::SigMismatch) => {
                respond(Content::FileSigChanged(FileSigChangedArgs {
                    sig: local_file.sig,
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

    match state.files.lock().await.get_mut(&args.id) {
        Some(local_file) => {
            match local_file.write_all(args.sig, &args.data).await {
                Ok(_) => {
                    respond(Content::FileWritten(FileWrittenArgs {
                        sig: local_file.sig,
                    }))
                    .await
                }
                Err(LocalFileWriteError::SigMismatch) => {
                    respond(Content::FileSigChanged(FileSigChangedArgs {
                        sig: local_file.sig,
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

pub async fn do_list_dir_contents<F, R>(
    _state: Arc<ServerState>,
    args: &DoListDirContentsArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("do_list_dir_contents: {:?}", args);

    match lookup_entries(&args.path).await {
        Ok(entries) => {
            respond(Content::DirContentsList(From::from(entries))).await
        }
        Err(x) => respond(Content::IoError(From::from(x))).await,
    }
}

async fn lookup_entries(path: &str) -> Result<Vec<DirEntry>, io::Error> {
    let mut entries = Vec::new();
    let mut dir_stream = fs::read_dir(path).await?;
    while let Some(entry) = dir_stream.next_entry().await? {
        let file_type = entry.file_type().await?;
        entries.push(DirEntry {
            path: entry.path().into_os_string().into_string().map_err(
                |_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "OS String does not contain valid unicode",
                    )
                },
            )?,
            is_file: file_type.is_file(),
            is_dir: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

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
                let x = state.files.lock().await;
                let local_file = x.get(&args.id).unwrap();
                assert_eq!(args.sig, local_file.sig);
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
                let x = state.files.lock().await;
                let local_file = x.get(&args.id).unwrap();
                assert_eq!(args.sig, local_file.sig);
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
    async fn do_read_file_should_send_contents_if_read_successful() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let file_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let id = 999;
        let sig = 12345;
        let mut file = tempfile::tempfile().unwrap();

        use std::io::Write;
        file.write_all(&file_data).unwrap();
        file.flush().unwrap();

        state.files.lock().await.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file),
                path: "".into(),
            },
        );

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

        let id = 999;
        let sig = 12345;
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

        state.files.lock().await.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file),
                path: "".into(),
            },
        );

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

        let id = 999;
        let cur_sig = 12345;
        let file = tempfile::tempfile().unwrap();
        state.files.lock().await.insert(
            id,
            LocalFile {
                id,
                sig: cur_sig,
                file: tokio::fs::File::from_std(file),
                path: "".into(),
            },
        );

        do_read_file(
            Arc::clone(&state),
            &DoReadFileArgs { id, sig: 99999 },
            |c| {
                content = Some(c);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        match content.unwrap() {
            Content::FileSigChanged(FileSigChangedArgs { sig }) => {
                assert_eq!(sig, cur_sig);
            }
            x => panic!("Bad content: {:?}", x),
        }
    }

    #[tokio::test]
    async fn do_write_file_should_send_success_if_write_successful() {
        let state = Arc::new(ServerState::default());
        let mut content: Option<Content> = None;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let id = 999;
        let sig = 12345;
        let mut file = tempfile::tempfile().unwrap();
        state.files.lock().await.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file.try_clone().unwrap()),
                path: "".into(),
            },
        );

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

        let id = 999;
        let sig = 12345;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        let tmp_file = tempfile::NamedTempFile::new().unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(tmp_file.path())
            .unwrap();
        state.files.lock().await.insert(
            id,
            LocalFile {
                id,
                sig,
                file: tokio::fs::File::from_std(file),
                path: "".into(),
            },
        );

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

        let id = 999;
        let cur_sig = 12345;
        let file = tempfile::tempfile().unwrap();
        state.files.lock().await.insert(
            id,
            LocalFile {
                id,
                sig: cur_sig,
                file: tokio::fs::File::from_std(file),
                path: "".into(),
            },
        );

        do_write_file(
            Arc::clone(&state),
            &DoWriteFileArgs {
                id,
                sig: 99999,
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
            Content::FileSigChanged(FileSigChangedArgs { sig }) => {
                assert_eq!(sig, cur_sig);
            }
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
