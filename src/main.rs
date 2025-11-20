use std::env;
use std::process;
use std::sync::Arc;
use std::sync::LazyLock;

use axum::Router;
use axum::middleware;
use axum::routing;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;
use tracing::Level;

mod handler;

// All closures are unwrapped to force panic on startup for any missing
// environment variables.
static DATABASE_URL: LazyLock<String> = LazyLock::new(|| env::var("DATABASE_URL").unwrap());
static PORT_NUMBER: LazyLock<String> = LazyLock::new(|| env::var("PORT").unwrap());
static ACCESS_SECRET: LazyLock<String> = LazyLock::new(|| env::var("JWT_ACCESS_SECRET").unwrap());
static REFRESH_SECRET: LazyLock<String> = LazyLock::new(|| env::var("JWT_REFRESH_SECRET").unwrap());

/// Dereference (initialise) the necessary environment variables. I'd rather
/// this panic on startup before any real "runtime" happens.
fn initialise_environment() {
  let _ = *DATABASE_URL;
  let _ = *PORT_NUMBER;
  let _ = *ACCESS_SECRET;
  let _ = *REFRESH_SECRET;

  tracing::info!("environment initialised");
}

/// Creates a router with all middleware, layers and shared state. Used within
/// handler tests for requests.
async fn create_app() -> anyhow::Result<Router> {
  let pool = PgPoolOptions::new().connect(&*DATABASE_URL).await?;
  let shared_pool = Arc::new(pool);

  let auth_layer = middleware::from_fn_with_state(shared_pool.clone(), handler::protected);
  let tracing_layer = TraceLayer::new_for_http();
  let cookie_layer = CookieManagerLayer::new();

  let app = Router::new()
    .route("/", routing::get(handler::index))
    .layer(auth_layer)
    .route("/login", routing::post(handler::login))
    .layer(tracing_layer)
    .layer(cookie_layer)
    .with_state(shared_pool);

  Ok(app)
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
    .with_target(false)
    .compact()
    .init();

  initialise_environment();

  let Ok(app) = create_app()
    .await
    .inspect_err(|err| tracing::error!("{err:#}"))
  else {
    process::exit(1)
  };

  let host = format!("0.0.0.0:{}", &*PORT_NUMBER);
  let Ok(listener) = TcpListener::bind(&host)
    .await
    .inspect_err(|err| tracing::error!("{err:#}"))
  else {
    process::exit(1)
  };

  tracing::info!("Starting at: {host}...");
  // On error, the TCP listener sleeps (for a second) without returning an error
  // so we unwrap the result regardless.
  axum::serve(listener, app).await.unwrap();
}
