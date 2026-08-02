use std::future::Future;
use std::pin::Pin;

use crate::request::Request;
use crate::response::Response;

/// Boxed future returned by async handlers.
pub type HandlerFuture = Pin<Box<dyn Future<Output = Response> + Send>>;

/// Language-neutral request handler. May be sync (via [`SyncHandler`]) or async.
pub trait Handler: Send + Sync {
    fn call(&self, req: Request) -> HandlerFuture;
}

/// Adapt a synchronous `Request -> Response` function into an async [`Handler`].
pub struct SyncHandler<F>(pub F);

impl<F> Handler for SyncHandler<F>
where
    F: Fn(Request) -> Response + Send + Sync + 'static,
{
    fn call(&self, req: Request) -> HandlerFuture {
        let response = (self.0)(req);
        Box::pin(async move { response })
    }
}

impl<F, Fut> Handler for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn call(&self, req: Request) -> HandlerFuture {
        Box::pin(self(req))
    }
}
