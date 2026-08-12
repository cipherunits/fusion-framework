use std::time::{Duration, Instant};

use axum::{routing::get, Json, Router};
use clap::Parser;
use reqwest::Client;

#[derive(Parser, Debug)]
struct Args {
    /// Total number of requests per framework
    #[arg(long, default_value_t = 20000)]
    requests: usize,
    /// Maximum in-flight requests
    #[arg(long, default_value_t = 200)]
    concurrency: usize,
    /// Target URL path
    #[arg(long, default_value = "/")]
    path: String,
}

fn percentile(sorted: &mut [Duration], pct: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx]
}

async fn run_load(url: String, requests: usize, concurrency: usize) -> (f64, Duration, Duration) {
    let client = Client::builder()
        .pool_max_idle_per_host(usize::MAX)
        .build()
        .expect("reqwest client");

    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(requests);
    let start = Instant::now();

    // Record all latencies to compute p50/p95.
    let latencies = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<Duration>::with_capacity(requests)));

    for _ in 0..requests {
        let permit = sem.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        let url = url.clone();
        let latencies = latencies.clone();
        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let t0 = Instant::now();
            let _ = client.get(url).send().await.expect("request");
            let dt = t0.elapsed();
            let mut guard = latencies.lock().await;
            guard.push(dt);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let total = start.elapsed();
    let rps = requests as f64 / total.as_secs_f64();

    let mut lats = latencies.lock().await.clone();
    lats.sort();
    let p50 = percentile(&mut lats, 50.0);
    let p95 = percentile(&mut lats, 95.0);
    (rps, p50, p95)
}

async fn run_fusion_core(port: u16, requests: usize, concurrency: usize, path: &str) -> anyhow::Result<()> {
    use fusion_core::{App as CoreApp, Response, SyncHandler};
    use serde_json::json;

    let mut app = CoreApp::new();
    app.route(
        "GET",
        path,
        SyncHandler(|_req| Response::json(200, &json!({ "ok": true }))),
    );

    let addr = format!("127.0.0.1:{port}");
    let server = tokio::spawn(async move {
        // fusion-core consumes app, so run as a task.
        let _ = app.listen_host_port("127.0.0.1", port).await;
    });

    // Best-effort warmup.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let url = format!("http://{addr}{path}");
    let (rps, p50, p95) = run_load(url, requests, concurrency).await;
    println!("fusion-core GET{path}: rps={rps:.1} p50={:?} p95={:?}", p50, p95);

    server.abort();
    Ok(())
}

async fn run_axum(port: u16, requests: usize, concurrency: usize, path: &str) -> anyhow::Result<()> {
    let std_listener = std::net::TcpListener::bind(("127.0.0.1", port))?;
    std_listener.set_nonblocking(true)?;
    let app = Router::new().route(path, get(|| async { Json(serde_json::json!({ "ok": true })) }));

    let server = tokio::spawn(async move {
        axum_server::from_tcp(std_listener)
            .serve(app.into_make_service())
            .await
            .ok();
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    let url = format!("http://127.0.0.1:{port}{path}");
    let (rps, p50, p95) = run_load(url, requests, concurrency).await;
    println!("axum GET{path}: rps={rps:.1} p50={:?} p95={:?}", p50, p95);

    server.abort();
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Run sequentially so the machine is not overloaded.
    run_fusion_core(3101, args.requests, args.concurrency, &args.path).await?;
    run_axum(3102, args.requests, args.concurrency, &args.path).await?;

    Ok(())
}

