use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use jsonwebtoken::Validation;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use sqlx::types::chrono;
use tower_cookies::Cookie;
use tower_cookies::Cookies;
use tower_cookies::cookie::time::OffsetDateTime;

use crate::ACCESS_SECRET;
use crate::REFRESH_SECRET;

const THIRTY_MINUTES_AS_SECS: i64 = 1_800;
const THIRTY_DAYS_AS_SECS: i64 = 2_592_000;
const ACCESS_COOKIE_NAME: &str = "access_token";
const REFRESH_COOKIE_NAME: &str = "refresh_token";

pub struct HandlerError(StatusCode);
type HandlerResult<T> = Result<T, HandlerError>;

impl IntoResponse for HandlerError {
  fn into_response(self) -> Response {
    self.0.into_response()
  }
}

impl From<StatusCode> for HandlerError {
  fn from(code: StatusCode) -> Self {
    Self(code)
  }
}

impl From<sqlx::Error> for HandlerError {
  fn from(err: sqlx::Error) -> Self {
    tracing::error!("[DATABASE] {err}");
    Self(StatusCode::INTERNAL_SERVER_ERROR)
  }
}

impl From<anyhow::Error> for HandlerError {
  fn from(err: anyhow::Error) -> Self {
    tracing::error!("[UNEXPECTED] {err}");
    Self(StatusCode::INTERNAL_SERVER_ERROR)
  }
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
  refresh_token: Option<String>,
  created_at: chrono::DateTime<chrono::Utc>,
  updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
  exp: usize,
  first_name: String,
}

impl Claims {
  fn new(cookie_name: &str, first_name: &str) -> Self {
    let first_name = first_name.to_owned();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let exp = match cookie_name {
      ACCESS_COOKIE_NAME => (now + THIRTY_MINUTES_AS_SECS) as usize,
      REFRESH_COOKIE_NAME => (now + THIRTY_DAYS_AS_SECS) as usize,
      _ => unreachable!(),
    };

    Self { exp, first_name }
  }

  fn from_token(secret: &str, token: &str) -> anyhow::Result<Self> {
    let validation = Validation::default();
    let secret = secret.as_bytes();
    let secret = DecodingKey::from_secret(&secret);

    jsonwebtoken::decode(token, &secret, &validation)
      .map(|data| data.claims)
      .map_err(anyhow::Error::from)
  }

  fn into_token(&self, secret: &str) -> anyhow::Result<String> {
    let header = Header::default();
    let secret = secret.as_bytes();
    let secret = EncodingKey::from_secret(&secret);

    jsonwebtoken::encode(&header, self, &secret).map_err(anyhow::Error::from)
  }

  fn into_cookie<'l>(self, cookie_name: &'l str, token: String) -> anyhow::Result<Cookie<'l>> {
    let base = Cookie::new(cookie_name, token);
    let expiry = OffsetDateTime::from_unix_timestamp(self.exp as i64)?;

    let cookie = Cookie::build(base)
      .http_only(true)
      .secure(true)
      .expires(expiry)
      .build();

    Ok(cookie)
  }
}

pub async fn index(Extension(user): Extension<Claims>) -> HandlerResult<impl IntoResponse> {
  Ok(user.first_name)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginPayload {
  first_name: String,
}

pub async fn login(
  cookies: Cookies,
  State(pool): State<Arc<PgPool>>,
  Json(payload): Json<LoginPayload>,
) -> HandlerResult<impl IntoResponse> {
  if cookies.get(ACCESS_COOKIE_NAME).is_some() {
    return Err(StatusCode::NO_CONTENT).map_err(HandlerError::from);
  }

  let user = get_user_by_name(&pool, &payload.first_name).await?;
  let first_name = &user.first_name;

  let access_claims = Claims::new(ACCESS_COOKIE_NAME, first_name);
  let access_token = access_claims.into_token(&*ACCESS_SECRET)?;
  let refresh_claims = Claims::new(REFRESH_COOKIE_NAME, first_name);
  let refresh_token = refresh_claims.into_token(&*REFRESH_SECRET)?;

  // The refresh token stored will be checked for when generating a new access
  // token from an existing refresh token.
  set_refresh_token_by_name(&pool, &refresh_token, first_name).await?;

  let access_cookie = access_claims.into_cookie(ACCESS_COOKIE_NAME, access_token)?;
  let refresh_cookie = refresh_claims.into_cookie(REFRESH_COOKIE_NAME, refresh_token)?;
  cookies.add(access_cookie);
  cookies.add(refresh_cookie);

  Ok(StatusCode::OK)
}

pub async fn protected(
  cookies: Cookies,
  State(pool): State<Arc<PgPool>>,
  mut request: Request,
  next: Next,
) -> HandlerResult<impl IntoResponse> {
  let refresh_token_from_cookie = cookies
    .get(REFRESH_COOKIE_NAME)
    .ok_or(StatusCode::UNAUTHORIZED)?
    .value()
    .to_string();

  let refresh_claims = Claims::from_token(&*REFRESH_SECRET, &refresh_token_from_cookie)?;
  let first_name = &refresh_claims.first_name;

  // At this point, we have a valid decoded refresh token so a missing access
  // token can be replaced using said decoded data.
  if cookies.get(ACCESS_COOKIE_NAME).is_none() {
    let access_claims = Claims::new(ACCESS_COOKIE_NAME, first_name);
    let access_token = access_claims.into_token(&*ACCESS_SECRET)?;
    let refresh_claims = Claims::new(REFRESH_COOKIE_NAME, first_name);
    let refresh_token = refresh_claims.into_token(&*REFRESH_SECRET)?;

    // Whenever the access token needs to be generated we also generate a new
    // refresh token. Then stored to ensure the old one expires.
    update_refresh_token(&pool, &refresh_token, &refresh_token_from_cookie).await?;

    let access_cookie = access_claims.into_cookie(ACCESS_COOKIE_NAME, access_token)?;
    let refresh_cookie = refresh_claims.into_cookie(REFRESH_COOKIE_NAME, refresh_token)?;
    cookies.add(access_cookie);
    cookies.add(refresh_cookie);
  }

  request.extensions_mut().insert(refresh_claims);
  Ok(next.run(request).await)
}

async fn get_user_by_name(pool: &PgPool, name: &str) -> HandlerResult<User> {
  let statement = r#"
    SELECT * FROM users
    WHERE first_name = $1
    LIMIT 1
  "#;

  sqlx::query_as(statement)
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(HandlerError::from)
}

async fn set_refresh_token_by_name(
  pool: &PgPool,
  token: &str,
  name: &str,
) -> HandlerResult<PgQueryResult> {
  let statement = r#"
    UPDATE users
    SET refresh_token = $1
    WHERE first_name = $2
  "#;

  sqlx::query(statement)
    .bind(token)
    .bind(name)
    .execute(pool)
    .await
    .map_err(HandlerError::from)
}

async fn update_refresh_token(
  pool: &PgPool,
  new_token: &str,
  old_token: &str,
) -> HandlerResult<PgQueryResult> {
  let statement = r#"
    UPDATE users
    SET refresh_token = $1
    WHERE refresh_token = $2
  "#;

  sqlx::query(statement)
    .bind(new_token)
    .bind(old_token)
    .execute(pool)
    .await
    .map_err(HandlerError::from)
}

#[cfg(test)]
mod tests {
  use std::sync::Once;

  use axum_test::TestResponse;
  use axum_test::TestServer;
  use serde_json::Value;

  use crate::create_app;
  use crate::setup_logging;

  use super::*;

  static LOGGING_INITIALISED: Once = Once::new();

  async fn setup() -> TestServer {
    LOGGING_INITIALISED.call_once(setup_logging);

    let app = create_app().await.unwrap();
    TestServer::builder().build(app).unwrap()
  }

  fn valid_payload() -> Value {
    serde_json::json!({ "firstName": "Alice" })
  }

  fn assert_cookies(response: &TestResponse, exists: bool) {
    let access_cookie = response.maybe_cookie(ACCESS_COOKIE_NAME);
    let refresh_cookie = response.maybe_cookie(REFRESH_COOKIE_NAME);

    assert_eq!(access_cookie.is_some(), exists);
    assert_eq!(refresh_cookie.is_some(), exists);
  }

  #[tokio::test]
  async fn login_valid_body() {
    let server = setup().await;
    let payload = valid_payload();

    let response = server.post("/login").json(&payload).await;

    assert_cookies(&response, true);
    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn login_invalid_body() {
    let server = setup().await;
    let payload = serde_json::json!({ "foo": "bar" });

    let response = server.post("/login").json(&payload).expect_failure().await;

    assert_cookies(&response, false);
    assert_eq!(response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
  }

  #[tokio::test]
  async fn unauthenticated_protected_route() {
    let server = setup().await;

    let response = server.get("/").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn authenticated_protected_route() {
    let server = setup().await;
    let payload = valid_payload();

    server.post("/login").json(&payload).save_cookies().await;
    let response = server.get("/").await;

    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn expired_access_token_protected_route() {
    let server = setup().await;
    let payload = valid_payload();

    let response = server.post("/login").json(&payload).await;
    let refresh_cookie = response.cookie(REFRESH_COOKIE_NAME);
    let response = server
      .get("/")
      .add_cookie(refresh_cookie)
      .save_cookies()
      .await;

    assert_cookies(&response, true);
    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn missing_refresh_token_with_access_token() {
    let server = setup().await;
    let payload = valid_payload();

    let response = server.post("/login").json(&payload).await;
    response.cookies().force_remove(REFRESH_COOKIE_NAME);
    let response = server.get("/").expect_failure().await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
  }
}
