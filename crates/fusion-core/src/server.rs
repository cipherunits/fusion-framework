use std::convert::Infallible;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use console::{Term, style};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::signal;

use crate::error::{Error, Result};
use crate::headers::apply_fingerprint_headers;
use crate::request::{Request, parse_query};
use crate::response::Response;
use crate::router::Router;

/// Options for [`listen_with`].
#[derive(Debug, Clone)]
pub struct ListenOptions {
    /// Add Fusion identity headers (`X-Powered-By`, …) on every response. Default: false.
    pub fingerprint: bool,
}

impl Default for ListenOptions {
    fn default() -> Self {
        Self { fingerprint: false }
    }
}

fn enable_colors() {
    // Touching Term initializes Windows VT processing when available.
    let _ = Term::stdout();
    let _ = Term::stderr();
    // Force ANSI even when stdout is not detected as a TTY (common under
    // language bindings like Python/Node).
    console::set_colors_enabled(true);
    console::set_colors_enabled_stderr(true);
}

fn color_status(status: u16) -> console::StyledObject<u16> {
    let styled = match status {
        200..=299 => style(status).green(),
        300..=399 => style(status).yellow(),
        400..=499 => style(status).red(),
        500..=599 => style(status).red(),
        _ => style(status).magenta(),
    };
    styled.force_styling(true)
}

pub async fn listen(router: Router, addr: SocketAddr) -> Result<()> {
    listen_with(router, addr, ListenOptions::default()).await
}

pub async fn listen_with(router: Router, addr: SocketAddr, options: ListenOptions) -> Result<()> {
    enable_colors();

    let listener = TcpListener::bind(addr).await?;
    let router = Arc::new(router);
    let options = Arc::new(options);

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                eprintln!(
                    "{}",
                    style("fusion: shutting down").yellow().force_styling(true)
                );
                break;
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                let io = TokioIo::new(stream);
                let router = Arc::clone(&router);
                let options = Arc::clone(&options);

                tokio::spawn(async move {
                    let peer = peer.to_string();
                    let service = service_fn(move |req| {
                        let router = Arc::clone(&router);
                        let options = Arc::clone(&options);
                        let peer = peer.clone();
                        async move { handle_request(router, options, peer, req).await }
                    });

                    if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                        eprintln!(
                            "{}",
                            style(format!("connection error: {err}"))
                                .red()
                                .force_styling(true)
                        );
                    }
                });
            }
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            eprintln!(
                "{}",
                style(format!("fusion: failed to listen for Ctrl+C: {err}"))
                    .red()
                    .force_styling(true)
            );
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                eprintln!(
                    "{}",
                    style(format!("fusion: failed to listen for SIGTERM: {err}"))
                        .red()
                        .force_styling(true)
                );
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn handle_request(
    router: Arc<Router>,
    options: Arc<ListenOptions>,
    peer: String,
    req: HyperRequest<Incoming>,
) -> std::result::Result<HyperResponse<Full<Bytes>>, Infallible> {
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let query = parse_query(req.uri().query().unwrap_or(""));
    let started = Instant::now();

    let headers = req
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_string(), v.to_string()))
        })
        .collect();

    let body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => Bytes::new(),
    };

    let request = Request::new(method.clone(), path.clone(), headers, body_bytes).with_query(query);

    let mut response = router.dispatch(request).await;
    if options.fingerprint {
        apply_fingerprint_headers(&mut response.headers);
    }

    let path_color = style(&path).blue().force_styling(true);
    let method_color = style(&method).cyan().force_styling(true);
    let status_color = color_status(response.status);
    let elapsed_ms = started.elapsed().as_millis();

    println!("{peer} {method_color} {path_color} -> {status_color} ({elapsed_ms}ms)");
    let _ = io::stdout().flush();
    Ok(to_hyper_response(response))
}
fn to_hyper_response(response: Response) -> HyperResponse<Full<Bytes>> {
    let mut builder = HyperResponse::builder().status(response.status);
    for (name, value) in &response.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    builder.body(Full::new(response.body)).unwrap_or_else(|_| {
        HyperResponse::builder()
            .status(500)
            .body(Full::new(Bytes::from_static(b"Internal Server Error")))
            .expect("fallback response")
    })
}

pub fn parse_addr(host: &str, port: u16) -> Result<SocketAddr> {
    format!("{host}:{port}")
        .parse()
        .map_err(|e| Error::InvalidAddress(format!("{e}")))
}
