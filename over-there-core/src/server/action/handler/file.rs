use crate::{
    msg::content::Content,
    server::{action::ActionError, file::LocalFile, state::ServerState},
};
use log::debug;
use rand::RngCore;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind};

pub fn file_do_open(
    state: &mut ServerState,
    path: String,
    create_if_missing: bool,
    write_access: bool,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!(
        "file_do_open: {} ; create: {} ; write: {}",
        path, create_if_missing, write_access
    );

    match OpenOptions::new()
        .create(create_if_missing)
        .write(write_access)
        .open(path)
    {
        Ok(file) => {
            let r = rand::thread_rng();
            let id = r.next_u32();
            let sig = r.next_u32();

            // Store the opened file so we can operate on it later
            state.files.insert(id, LocalFile { id, sig, file });

            respond(Content::FileOpened { id, sig })
        }
        Err(x) => {
            use std::error::Error;
            respond(Content::FileError {
                description: x.description().to_string(),
                error_kind: x.kind(),
            })
        }
    }
}

pub fn file_do_read(
    state: &mut ServerState,
    id: u32,
    sig: u32,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("file_do_read: id: {} ; sig: {}", id, sig);

    match state.files.get(&id) {
        Some(local_file) => {
            if local_file.sig == sig {
                respond(Content::FileContents {
                    data: {
                        use std::io::Read;
                        let mut buf = Vec::new();
                        local_file.file.read_to_end(&mut buf);
                        buf
                    },
                })
            } else {
                respond(Content::FileSigChanged)
            }
        }
        None => respond(Content::FileError {
            description: String::from(format!("No file open with id {}", id)),
            error_kind: ErrorKind::InvalidInput,
        }),
    }
}

pub fn file_do_write(
    state: &mut ServerState,
    id: u32,
    sig: u32,
    data: &[u8],
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!(
        "file_do_write: id: {} ; sig: {} ; data: {} bytes",
        id,
        sig,
        data.len()
    );

    match state.files.get_mut(&id) {
        Some(local_file) => {
            if local_file.sig == sig {
                use std::io::Write;
                match local_file.file.write_all(data) {
                    Ok(_) => {
                        let new_sig = rand::thread_rng().next_u32();
                        local_file.sig = new_sig;

                        respond(Content::FileWritten { sig: new_sig })
                    }
                    Err(x) => {
                        use std::error::Error;
                        respond(Content::FileError {
                            description: x.description().to_string(),
                            error_kind: x.kind(),
                        })
                    }
                }
            } else {
                respond(Content::FileSigChanged)
            }
        }
        None => respond(Content::FileError {
            description: String::from(format!("No file open with id {}", id)),
            error_kind: ErrorKind::InvalidInput,
        }),
    }
}

pub fn file_do_list(
    _state: &mut ServerState,
    path: String,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("file_do_list: path: {}", path);

    let lookup_entries = |path| -> Result<(), io::Error> {
        fs::read_dir(path)?
            .map(|res| res.map(|e| (e.file_type(), e.path())))
            .collect::<Result<Vec<_>, io::Error>>()
    };

    match lookup_entries(path) {
        Ok(entries) => respond(Content::FileList { entries }),
        Err(x) => {
            use std::error::Error;
            respond(Content::FileError {
                description: x.description().to_string(),
                error_kind: x.kind(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::action::test_utils::MockResponder;

    #[test]
    fn version_request_should_send_version_response() {
        let mut state = ServerState::default();
        let msg = Msg::from(Content::VersionRequest);
        let mut responder = MockResponder::default();

        let result = version_request(&mut state, &msg, &responder);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let outgoing_msg = Msg::from_slice(&responder.take_last_sent().unwrap()).unwrap();
        assert_eq!(outgoing_msg.parent_header, Some(msg.header));
        assert_eq!(
            outgoing_msg.content,
            Content::VersionResponse {
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
        );
    }
}
