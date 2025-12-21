use std::env::var;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::routing::post;
use axum::serve;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing::error;
use tracing::info;
use tracing_subscriber::fmt;

use crate::handler::auth::auth;
use crate::handler::auth::login;
use crate::handler::index;

mod handler;

#[derive(Clone)]
struct AppState {
  pool: Arc<PgPool>,
  secret: String,
}

/// Create a logger to STDOUT based on an environment variable or default.
///
/// NOTE: Used within the handler tests.
fn setup_logging() {
  let env_lvl = "DEBUG".to_string();
  let env_lvl = var("LOG_LEVEL").unwrap_or(env_lvl);
  let log_lvl = Level::from_str(&env_lvl).unwrap_or(Level::DEBUG);

  fmt()
    .with_max_level(log_lvl)
    .with_target(false)
    .compact()
    .init();

  info!("logging lvl: {log_lvl}");
}

/// Initialise the database pool, handlers and [`AppState`].
///
/// NOTE: Used within the handler tests.
async fn create_app() -> Result<Router> {
  let (db, secret) = (var("DATABASE_URL")?, var("TOKEN_SECRET")?);
  let pool = PgPoolOptions::new().connect(&db).await?;
  let pool = Arc::new(pool);

  let state = AppState { pool, secret };
  let auth_ware = from_fn_with_state(state.clone(), auth);
  let trace_lyr = TraceLayer::new_for_http();
  let cookie_lyr = CookieManagerLayer::new();

  let app = Router::new()
    .route("/", get(index))
    .layer(auth_ware)
    .route("/login", post(login))
    .layer(trace_lyr)
    .layer(cookie_lyr)
    .with_state(state);

  Ok(app)
}

#[tokio::main]
async fn main() {
  setup_logging();
  let Ok(app) = create_app().await.inspect_err(|err| error!("{err}")) else {
    exit(1)
  };

  let host = {
    let port = "8080".to_string();
    let port = var("PORT").unwrap_or(port);
    format!("0.0.0.0:{port}")
  };

  match TcpListener::bind(&host).await {
    Ok(listener) => {
      // NOTE: This can be unwrapped as the error handling is just to sleep.
      //
      // "Errors on the TCP socket will be handled by sleeping for a short while
      // (currently, one second)" - axum::serve
      info!("starting at: {host}");
      serve(listener, app).await.unwrap();
    }
    Err(err) => {
      error!("{err}");
      exit(1)
    }
  }
}
