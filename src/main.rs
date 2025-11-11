use std::{env, process, sync::Arc};

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
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

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

/// Basic route handler to query all users from the database. NOTE: It's so
/// basic that there's no auth, rate limiting etc.
async fn index(State(pool): State<Arc<PgPool>>) -> impl IntoResponse {
  let Ok(users) = sqlx::query_as::<_, User>("SELECT * FROM users")
    .fetch_all(&*pool)
    .await
  else {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  };

  let mut response = (StatusCode::OK, Json(users)).into_response();
  response.headers_mut().insert(
    "cache-control",
    HeaderValue::from_static("private, max-age=3600"),
  );

  response
}

/// Build a [`TcpListener`] and [`Router`] with tracing, database pool and route
/// handlers. NOTE: All errors are bubbled up to be logged.
async fn startup() -> anyhow::Result<(TcpListener, Router)> {
  let database_url = env::var("DATABASE_URL")?;
  let pool = PgPoolOptions::new().connect(&database_url).await?;

  let tracing_layer = TraceLayer::new_for_http()
    .on_response(DefaultOnResponse::new().level(Level::INFO))
    .on_request(DefaultOnRequest::new().level(Level::INFO));

  let app = Router::new()
    .route("/", get(index))
    .layer(tracing_layer)
    .with_state(Arc::new(pool));

  let host = format!("0.0.0.0:{}", env::var("PORT").unwrap_or("8080".into()));
  let listener = TcpListener::bind(&host).await?;

  tracing::info!("Starting at: {host}");
  Ok((listener, app))
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .with_target(false)
    .compact()
    .init();

  let Ok((listener, app)) = startup()
    .await
    .inspect_err(|err| tracing::error!("{err:#}"))
  else {
    process::exit(1);
  };

  // On error, the TCP listener sleeps (for a second) without returning an error
  // so we unwrap the result regardless.
  axum::serve(listener, app).await.unwrap();
}
