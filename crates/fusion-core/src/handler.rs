use crate::request::Request;
use crate::response::Response;

pub trait Handler: Send + Sync {
    fn call(&self, req: Request) -> Response;
}

impl<F> Handler for F
where
    F: Fn(Request) -> Response + Send + Sync,
{
    fn call(&self, req: Request) -> Response {
        self(req)
    }
}
