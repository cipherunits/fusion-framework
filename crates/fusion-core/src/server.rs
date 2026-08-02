use std::convert::Infallible;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::signal;

use crate::error::{Error, Result};
use crate::request::Request;
use crate::response::Response;
use crate::router::Router;

pub async fn listen(router: Router, addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let router = Arc::new(router);

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                eprintln!("fusion: shutting down");
                break;
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                let io = TokioIo::new(stream);
                let router = Arc::clone(&router);

                tokio::spawn(async move {
                    let peer = peer.to_string();
                    let service = service_fn(move |req| {
                        let router = Arc::clone(&router);
                        let peer = peer.clone();
                        async move { handle_request(router, peer, req).await }
                    });

                    if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                        eprintln!("connection error: {err}");
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
            eprintln!("fusion: failed to listen for Ctrl+C: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                eprintln!("fusion: failed to listen for SIGTERM: {err}");
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
    peer: String,
    req: HyperRequest<Incoming>,
) -> std::result::Result<HyperResponse<Full<Bytes>>, Infallible> {
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
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

    let request = Request::new(method.clone(), path.clone(), headers, body_bytes);
    let response = router.dispatch(request);
    let status = response.status;
    let elapsed_ms = started.elapsed().as_millis();

    println!("{peer} {method} {path} -> {status} ({elapsed_ms}ms)");
    let _ = io::stdout().flush();

    Ok(to_hyper_response(response))
}

fn to_hyper_response(response: Response) -> HyperResponse<Full<Bytes>> {
    let mut builder = HyperResponse::builder().status(response.status);

    for (name, value) in &response.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }

    builder
        .body(Full::new(response.body))
        .unwrap_or_else(|_| {
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
