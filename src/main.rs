use std::{env, sync::Arc};

use axum::{
  Json, Router,
  extract::State,
  http::{HeaderValue, StatusCode},
  response::IntoResponse,
  routing::get,
};
use serde::Serialize;
use sqlx::{PgPool, postgres::PgPoolOptions, prelude::FromRow};
use tokio::net::TcpListener;

const CACHE_HEADER: HeaderValue = HeaderValue::from_static("private, max-age=3600");

macro_rules! log {
  (INFO, $($txt:tt)*) => {
    println!("[\x1b[1m\x1b[32mINFO \x1b[0m] {}", format!($($txt)*))
  };
  (ERROR, $($txt:tt)*) => {
    eprintln!("[\x1b[1m\x1b[31mERROR\x1b[0m] {}", format!($($txt)*))
  };
  (ABORT, $($txt:tt)*) => {
    eprintln!("[\x1b[1m\x1b[31mERROR\x1b[0m] {}", format!($($txt)*));
    std::process::exit(1)
  };
  ($($txt:tt)*) => {
    println!("[\x1b[1m\x1b[34mDEBUG\x1b[0m] {}", format!($($txt)*))
  };
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct User {
  id: i32,
  age: i32,
  is_pro: bool,
  mobile: String,
  last_name: String,
  first_name: String,
}

async fn index(State(pool): State<Arc<PgPool>>) -> impl IntoResponse {
  log!(INFO, "GET /");

  let Ok(users) = sqlx::query_as::<_, User>("SELECT * FROM users")
    .fetch_all(&*pool)
    .await
    .inspect_err(|err| log!(ERROR, "GET / {err}"))
  else {
    return StatusCode::NOT_FOUND.into_response();
  };

  log!("Users: {}", users.len());
  let mut response = (StatusCode::OK, Json(users)).into_response();
  response.headers_mut().insert("cache-control", CACHE_HEADER);

  response
}

#[tokio::main]
async fn main() {
  let host = env::var("PORT")
    .map(|var| format!("0.0.0.0:{}", var))
    .unwrap_or_else(|_| "0.0.0.0:8080".into());

  let Ok(database) = env::var("DATABASE_URL") else {
    log!(ABORT, "DATABASE_URL not set");
  };

  let Ok(pool) = PgPoolOptions::new()
    .connect(&database)
    .await
    .inspect_err(|err| log!(ERROR, "Connecting to database: {err}"))
  else {
    return;
  };

  let shared_pool = Arc::new(pool);
  let app = Router::new().route("/", get(index)).with_state(shared_pool);

  let Ok(listener) = TcpListener::bind(&host).await else {
    log!(ABORT, "Binding to: {host}");
  };

  log!(INFO, "Starting at: {host}...");
  if let Err(err) = axum::serve(listener, app).await {
    log!(ABORT, "Starting server: {}", err);
  }
}
