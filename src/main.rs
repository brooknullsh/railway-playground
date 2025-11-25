use std::env;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::routing::post;
use axum::serve;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing::error;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

use crate::handler::index;
use crate::handler::login;
use crate::handler::protected;

mod handler;

fn setup_logging() {
  let log_level = env!("LOG_LEVEL");
  let log_level = Level::from_str(log_level).unwrap_or(Level::DEBUG);
  let filter = EnvFilter::from_default_env();

  fmt()
    .with_env_filter(filter)
    .with_max_level(log_level)
    .with_target(false)
    .compact()
    .init();

  info!("logging initialised with {}", log_level);
}

async fn create_app() -> anyhow::Result<Router> {
  let connection = env!("DATABASE_URL");
  let pool = PgPoolOptions::new().connect(connection).await?;
  let shared_pool = Arc::new(pool);

  let auth_middleware = from_fn_with_state(shared_pool.clone(), protected);
  let tracing_layer = TraceLayer::new_for_http();
  let cookie_layer = CookieManagerLayer::new();

  let app = Router::new()
    .route("/", get(index))
    .layer(auth_middleware)
    .route("/login", post(login))
    .layer(tracing_layer)
    .layer(cookie_layer)
    .with_state(shared_pool);

  Ok(app)
}

#[tokio::main]
async fn main() {
  setup_logging();

  let Ok(app) = create_app()
    .await
    .inspect_err(|err| error!("[SETUP] {err:#}"))
  else {
    exit(1)
  };

  let port = env!("PORT");
  let host = format!("0.0.0.0:{}", port);

  let Ok(listener) = TcpListener::bind(&host)
    .await
    .inspect_err(|err| error!("[SETUP] {err:#}"))
  else {
    exit(1)
  };

  // On error, the TCP listener sleeps (for a second) without returning an error
  // so we unwrap the result regardless.
  info!("Starting at: {host}...");
  serve(listener, app).await.unwrap();
}
