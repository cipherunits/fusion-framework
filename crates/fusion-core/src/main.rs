use fusion_core::{App, Response};

#[tokio::main]
async fn main() {
    let mut app = App::new();
    app.route("GET", "/", |_req| Response::text(200, "ok"));
    app.route("GET", "/api/[name]/{id}", |req| {
        let name = req.params.get("name").map(String::as_str).unwrap_or("-");
        let id = req.params.get("id").map(String::as_str).unwrap_or("-");
        Response::text(200, format!("hello {name}/{id}"))
    });

    let addr = "127.0.0.1:3000";
    println!("fusion-core listening on http://{addr}");
    if let Err(err) = app.listen_host_port("127.0.0.1", 3000).await {
        eprintln!("server error: {err}");
    }
}
