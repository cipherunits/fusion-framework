mod error;
mod handler;
mod request;
mod response;
mod router;
mod server;

pub use error::{Error, Result};
pub use handler::Handler;
pub use request::Request;
pub use response::Response;
pub use router::Router;

use std::net::SocketAddr;

/// Language-neutral HTTP application. Owns routing; `listen` is driven by hyper.
#[derive(Clone, Default)]
pub struct App {
    router: Router,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    pub fn route(&mut self, method: &str, path: &str, handler: impl Handler + 'static) {
        self.router.route(method, path, handler);
    }

    pub fn router(&self) -> &Router {
        &self.router
    }

    pub fn into_router(self) -> Router {
        self.router
    }

    pub async fn listen(self, addr: SocketAddr) -> Result<()> {
        server::listen(self.router, addr).await
    }

    pub async fn listen_host_port(self, host: &str, port: u16) -> Result<()> {
        let addr = server::parse_addr(host, port)?;
        self.listen(addr).await
    }
}
