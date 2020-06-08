use crate::{reply, request, server::state::ServerState};
use log::debug;
use std::sync::Arc;

pub async fn internal_debug(
    state: Arc<ServerState>,
    _args: &request::InternalDebugArgs,
) -> reply::InternalDebugArgs {
    debug!("internal_debug_request");

    let mut output = vec![];

    output.extend_from_slice(state.internal_debug().await.as_bytes());

    reply::InternalDebugArgs { output }
}
