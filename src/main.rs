use std::env::var;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;

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

use crate::handler::auth;
use crate::handler::index;
use crate::handler::login;

mod handler;

#[derive(Clone)]
struct AppState {
  pool: Arc<PgPool>,
  acc_secret: String,
  ref_secret: String,
}

fn setup_logging() {
  let env_lvl = "DEBUG".to_string();
  let env_lvl = var("LOG_LEVEL").unwrap_or(env_lvl);
  let log_lvl = Level::from_str(&env_lvl).unwrap_or(Level::DEBUG);

  fmt()
    .with_max_level(log_lvl)
    .with_target(false)
    .compact()
    .init();

  info!("logging lvl: {}", log_lvl);
}

async fn create_app() -> anyhow::Result<Router> {
  let db = var("DATABASE_URL")?;
  let acc_secret = var("ACCESS_SECRET")?;
  let ref_secret = var("REFRESH_SECRET")?;

  let pool = PgPoolOptions::new().connect(&db).await?;
  let pool = Arc::new(pool);
  let state = AppState {
    pool,
    acc_secret,
    ref_secret,
  };

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

  let Ok(app) = create_app()
    .await
    .inspect_err(|err| error!("[SETUP] {err:#}"))
  else {
    exit(1)
  };

  let port = "8080".to_string();
  let port = var("PORT").unwrap_or(port);
  let host = format!("0.0.0.0:{}", port);

  let Ok(listener) = TcpListener::bind(&host)
    .await
    .inspect_err(|err| error!("[SETUP] {err:#}"))
  else {
    exit(1)
  };

  // On error, the TCP listener sleeps (for a second) without returning an error
  // so unwrap the result regardless.
  info!("starting at: {host}");
  serve(listener, app).await.unwrap();
}
