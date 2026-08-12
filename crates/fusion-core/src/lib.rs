mod api_context;
mod coerce;
mod dispatch;
mod error;
mod handler;
mod http_error;
mod naming;
mod request;
mod response;
mod router;
mod serialize;
mod server;
mod settings;
mod status;

pub use api_context::ApiContext;
pub use coerce::{ParamKind, coerce_param, param_kind_from_name};
pub use dispatch::{ParamSpec, bind_args, build_response, parse_json_object, BODY_METHODS};
pub use error::{Error, Result};
pub use handler::{Handler, HandlerFuture, SyncHandler};
pub use http_error::HttpError;
pub use naming::{HTTP_METHODS, api_resource_name, resolve_route_path};
pub use request::{parse_query, Request};
pub use response::Response;
pub use router::Router;
pub use serialize::{is_response_envelope, response_from_value};
pub use settings::Settings;
pub use status::{
    HTTP_STATUS_CODES, is_client_error, is_informational, is_redirect, is_server_error, is_success,
};

use std::net::SocketAddr;

/// Language-neutral HTTP application. Owns routing; `listen` is driven by hyper.
#[derive(Clone, Default)]
pub struct App {
    router: Router,
    settings: Settings,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            settings: Settings::new(),
        }
    }

    pub fn with_settings(settings: Settings) -> Self {
        Self {
            router: Router::new(),
            settings,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
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

    /// Listen using host/port from loaded settings.
    pub async fn listen_from_settings(self) -> Result<()> {
        let host = self.settings.host();
        let port = self.settings.port();
        self.listen_host_port(&host, port).await
    }
}
