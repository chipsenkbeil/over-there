use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod handler;
pub mod manager;
pub mod utils;
use utils::Either;

#[derive(Serialize, Deserialize, Debug)]
pub struct Msg {
    /// ID associated with a request or response
    id: u32,

    /// IDs in the chain of communication (oldest to newest)
    origin: Vec<u32>,

    /// Represents the request or response associated with the message
    req_or_res: Either<Request, Response>,
}

impl Msg {
    pub fn new(id: u32, origin: Vec<u32>, req_or_res: Either<Request, Response>) -> Self {
        Msg {
            id,
            origin,
            req_or_res,
        }
    }

    pub fn from_request(id: u32, origin: Vec<u32>, req: Request) -> Self {
        Self::new(id, origin, Either::Left(req))
    }

    pub fn from_response(id: u32, origin: Vec<u32>, res: Response) -> Self {
        Self::new(id, origin, Either::Right(res))
    }

    pub fn new_from_parent(id: u32, req_or_res: Either<Request, Response>, parent: &Msg) -> Self {
        let mut origin = parent.origin.clone();
        origin.append(&mut vec![parent.id]);
        Msg {
            id,
            origin,
            req_or_res,
        }
    }

    pub fn is_request(&self) -> bool {
        self.get_request().is_some()
    }

    pub fn get_request(&self) -> Option<&Request> {
        self.req_or_res.get_left()
    }

    pub fn is_response(&self) -> bool {
        self.get_response().is_some()
    }

    pub fn get_response(&self) -> Option<&Response> {
        self.req_or_res.get_right()
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    pub fn from_vec(v: &Vec<u8>) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(v)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    /// Make sure daemon is alive
    HeartbeatRequest,

    /// Request version
    VersionResponse,

    /// Request capabilities
    CapabilitiesResponse,

    /// List all files, directories, etc. at a path
    ///
    /// Path
    ListFilesRequest(String),

    /// Write the contents of a file
    ///
    /// Path, Contents
    WriteFileRequest(String, String),

    /// Read the contents of a file
    ///
    /// Path, Start (base 0), Total Bytes
    ReadFileRequest(String, u32, u32),

    /// Execute a command, potentially returning the completed output
    ///
    /// Args: Command, Args, WantStdOut, WantStdErr
    ExecRequest(String, Vec<String>, bool, bool),

    /// Execute a command, potentially streaming the live output
    ///
    /// Command, Args, WantStdOut, WantStdErr
    ExecStreamRequest(String, Vec<String>, bool, bool),

    /// TODO: Think of format for hopping from one instance to another
    ///       in case of client -> server 1 -> server 2
    ///
    /// Server 2 Address, Message to forward
    ForwardRequest(String, Box<Request>),

    /// Key-value map for custom requests
    ///
    /// Args: Map
    CustomRequest(HashMap<String, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    /// Report alive status
    HeartbeatResponse,

    /// Report version
    VersionResponse(String),

    /// Report capabilities
    CapabilitiesResponse(Vec<String>),

    /// Generic error reponse used upon failing
    ///
    /// Request, Error Message
    ErrorResponse(Request, String),

    /// List all files, directories, etc. at a path
    ///
    /// Paths
    ListFilesResponse(Vec<String>),

    /// Write the contents of a file
    ///
    /// Bytes written
    WriteFileResponse(u32),

    /// Read the contents of a file
    ///
    /// Bytes read
    ReadFileResponse(Vec<u8>),

    /// Execute a command, potentially returning the completed output
    ///
    /// ErrCode, StdOut, StdErr
    ExecResponse(u32, Option<String>, Option<String>),

    /// Execute a command, potentially streaming the live output
    ///
    /// ErrCode (none if still running), StdOut, StdErr
    ExecStreamResponse(Option<u32>, Option<String>, Option<String>),

    /// TODO: Think of format for hopping from one instance to another
    ///       in case of client -> server 1 -> server 2
    ///
    /// Client Address, Message to pass back
    ForwardResponse(String, Box<Response>),

    /// Key-value map for custom responses
    ///
    /// Args: Map
    CustomResponse(HashMap<String, String>),
}
