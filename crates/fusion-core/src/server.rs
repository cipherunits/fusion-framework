use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::error::{Error, Result};
use crate::request::Request;
use crate::response::Response;
use crate::router::Router;

pub async fn listen(router: Router, addr: SocketAddr) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let router = Arc::new(router);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let router = Arc::clone(&router);

        tokio::spawn(async move {
            let service = service_fn(move |req| {
                let router = Arc::clone(&router);
                async move { handle_request(router, req).await }
            });

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("connection error: {err}");
            }
        });
    }
}

async fn handle_request(
    router: Arc<Router>,
    req: HyperRequest<Incoming>,
) -> std::result::Result<HyperResponse<Full<Bytes>>, Infallible> {
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();

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

    let request = Request::new(method, path, headers, body_bytes);
    let response = router.dispatch(request);
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
