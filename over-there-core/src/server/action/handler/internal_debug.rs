use crate::{
    msg::content::{internal_debug::InternalDebugArgs, Content},
    server::{action::ActionError, state::ServerState},
};
use log::debug;
use std::future::Future;
use std::sync::Arc;

pub async fn internal_debug<F, R>(
    state: Arc<ServerState>,
    args: &InternalDebugArgs,
    respond: F,
) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("internal_debug_request");

    let mut output = vec![];

    output.extend_from_slice(state.internal_debug().await.as_bytes());

    respond(Content::InternalDebug(InternalDebugArgs {
        input: args.input.clone(),
        output,
    }))
    .await
}
