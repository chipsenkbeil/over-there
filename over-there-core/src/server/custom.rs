use crate::{reply, request};
use futures::future::BoxFuture;
use std::fmt;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type CustomHandlerFunc = Box<
    dyn FnMut(
            request::CustomArgs,
        ) -> BoxFuture<
            'static,
            Result<reply::CustomArgs, Box<dyn std::error::Error>>,
        > + Send,
>;

#[derive(Clone)]
pub struct CustomHandler {
    f: Arc<Mutex<CustomHandlerFunc>>,
}

impl fmt::Debug for CustomHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomHandler").finish()
    }
}

impl CustomHandler {
    pub fn new(f: CustomHandlerFunc) -> Self {
        Self {
            f: Arc::new(Mutex::new(f)),
        }
    }

    pub async fn invoke(
        &self,
        args: request::CustomArgs,
    ) -> Result<reply::CustomArgs, Box<dyn std::error::Error>> {
        let f = &mut *self.f.lock().await;
        f(args).await
    }

    pub fn new_unimplemented() -> Self {
        Self::from(|_| async { Ok(reply::CustomArgs { data: vec![] }) })
    }
}

impl<F, R> From<F> for CustomHandler
where
    F: FnMut(request::CustomArgs) -> R + Send + 'static,
    R: Future<Output = Result<reply::CustomArgs, Box<dyn std::error::Error>>>
        + Send
        + 'static,
{
    fn from(mut f: F) -> Self {
        use futures::future::FutureExt;

        Self::new(Box::new(move |req| f(req).boxed()))
    }
}
