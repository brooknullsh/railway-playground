use std::{env, process, sync::Arc};

use axum::{Router, middleware, routing};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

mod handler;

async fn startup() -> anyhow::Result<(TcpListener, Router)> {
  let url = env::var("DATABASE_URL")?;
  let pool = PgPoolOptions::new().connect(&url).await?;

  let cookie_layer = CookieManagerLayer::new();
  let tracing_layer = TraceLayer::new_for_http()
    .on_response(DefaultOnResponse::new().level(Level::INFO))
    .on_request(DefaultOnRequest::new().level(Level::INFO));

  let shared_pool = Arc::new(pool);
  let auth_layer = middleware::from_fn_with_state(shared_pool.clone(), handler::protected);

  let app = Router::new()
    .route("/", routing::get(handler::index))
    .layer(auth_layer)
    .route("/login", routing::post(handler::login))
    .layer(tracing_layer)
    .layer(cookie_layer)
    .with_state(shared_pool);

  let port = env::var("PORT").unwrap_or("8080".to_string());
  let host = format!("0.0.0.0:{}", port);

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
    process::exit(1)
  };

  // On error, the TCP listener sleeps (for a second) without returning an error
  // so we unwrap the result regardless.
  axum::serve(listener, app).await.unwrap();
}
