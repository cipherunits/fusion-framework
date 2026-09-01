use fusion_core::{App, Request, Response, SyncHandler};

#[tokio::main]
async fn main() {
    let mut app = App::new();
    app.route(
        "GET",
        "/",
        SyncHandler(|_req: Request| Response::text(200, "ok")),
    );
    app.route(
        "GET",
        "/api/[module]/{id}",
        SyncHandler(|req: Request| {
            let module = req.params.get("module").map(|s| s.as_str()).unwrap_or("-");
            let id = req.params.get("id").map(|s| s.as_str()).unwrap_or("-");
            Response::text(200, format!("hello {module}/{id}"))
        }),
    );

    let addr = "127.0.0.1:3000";
    println!("fusion-core listening on http://{addr}");
    if let Err(err) = app.listen_host_port("127.0.0.1", 3000).await {
        eprintln!("server error: {err}");
    }
}
