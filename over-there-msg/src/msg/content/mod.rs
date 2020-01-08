pub mod request;
pub mod response;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Content {
    Request(request::Request),
    Response(response::Response),
}

//
// GENERAL REQUEST/RESPONSE CONVERSIONS
//

impl From<request::Request> for Content {
    fn from(request: request::Request) -> Self {
        Self::Request(request)
    }
}

impl From<response::Response> for Content {
    fn from(response: response::Response) -> Self {
        Self::Response(response)
    }
}

//
// SPECIFIC REQUEST CONVERSIONS
//

impl From<request::standard::StandardRequest> for Content {
    fn from(r: request::standard::StandardRequest) -> Self {
        Self::from(request::Request::from(r))
    }
}

#[cfg(feature = "custom")]
impl From<request::custom::CustomRequest> for Content {
    fn from(r: request::custom::CustomRequest) -> Self {
        Self::from(request::Request::from(r))
    }
}

#[cfg(feature = "exec")]
impl From<request::exec::ExecRequest> for Content {
    fn from(r: request::exec::ExecRequest) -> Self {
        Self::from(request::Request::from(r))
    }
}

#[cfg(feature = "forward")]
impl From<request::forward::ForwardRequest> for Content {
    fn from(r: request::forward::ForwardRequest) -> Self {
        Self::from(request::Request::from(r))
    }
}

#[cfg(feature = "file-system")]
impl From<request::file_system::FileSystemRequest> for Content {
    fn from(r: request::file_system::FileSystemRequest) -> Self {
        Self::from(request::Request::from(r))
    }
}

//
// SPECIFIC RESPONSE CONVERSIONS
//

impl From<response::standard::StandardResponse> for Content {
    fn from(r: response::standard::StandardResponse) -> Self {
        Self::from(response::Response::from(r))
    }
}

#[cfg(feature = "custom")]
impl From<response::custom::CustomResponse> for Content {
    fn from(r: response::custom::CustomResponse) -> Self {
        Self::from(response::Response::from(r))
    }
}

#[cfg(feature = "exec")]
impl From<response::exec::ExecResponse> for Content {
    fn from(r: response::exec::ExecResponse) -> Self {
        Self::from(response::Response::from(r))
    }
}

#[cfg(feature = "forward")]
impl From<response::forward::ForwardResponse> for Content {
    fn from(r: response::forward::ForwardResponse) -> Self {
        Self::from(response::Response::from(r))
    }
}

#[cfg(feature = "file-system")]
impl From<response::file_system::FileSystemResponse> for Content {
    fn from(r: response::file_system::FileSystemResponse) -> Self {
        Self::from(response::Response::from(r))
    }
}
