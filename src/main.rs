use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;
use tokio::net::TcpListener;

const HOST_ADDRESS: &str = "0.0.0.0:3000";

macro_rules! log {
  (INFO, $($txt:tt)*) => {
    println!("[\x1b[1m\x1b[32mINFO \x1b[0m] {}", format!($($txt)*))
  };
  (ABORT, $($txt:tt)*) => {
    eprintln!("[\x1b[1m\x1b[31mERROR\x1b[0m] {}", format!($($txt)*));
    std::process::exit(1);
  };
}

#[derive(Serialize)]
struct ExampleResponse<'l> {
  body: &'l str,
}

async fn index() -> impl IntoResponse {
  log!(INFO, "GET /");

  let response = ExampleResponse {
    body: "Hello, world!",
  };

  (StatusCode::OK, Json(response)).into_response()
}

#[tokio::main]
async fn main() {
  let app = Router::new().route("/", get(index));

  let Ok(listener) = TcpListener::bind(HOST_ADDRESS).await else {
    log!(ABORT, "Binding to: {HOST_ADDRESS}");
  };

  log!(INFO, "Starting at: {HOST_ADDRESS}...");
  if let Err(err) = axum::serve(listener, app).await {
    log!(ABORT, "Starting server: {}", err);
  }
}
