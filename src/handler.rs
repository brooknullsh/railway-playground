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
use jsonwebtoken::TokenData;
use jsonwebtoken::Validation;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::prelude::FromRow;
use sqlx::types::chrono;
use tower_cookies::Cookie;
use tower_cookies::Cookies;
use tower_cookies::cookie::time::OffsetDateTime;

use crate::ACCESS_SECRET;
use crate::REFRESH_SECRET;

const THIRTY_MINUTES_AS_SECS: i64 = 1_800;
const THIRTY_DAYS_AS_SECS: i64 = 2_592_000;

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

#[derive(PartialEq)]
enum TokenKind {
  Access,
  Refresh,
}

impl TokenKind {
  fn to_cookie_name<'l>(self) -> &'l str {
    match self {
      TokenKind::Access => "access_token",
      TokenKind::Refresh => "refresh_token",
    }
  }

  fn to_secret_as_bytes(self) -> Vec<u8> {
    let key = match self {
      TokenKind::Access => &*ACCESS_SECRET,
      TokenKind::Refresh => &*REFRESH_SECRET,
    };

    key.clone().into_bytes()
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

#[derive(Clone)]
pub struct UserContext {
  first_name: String,
}

pub async fn index(Extension(user): Extension<UserContext>) -> HandlerResult<impl IntoResponse> {
  Ok(user.first_name)
}

#[derive(Serialize, Deserialize)]
struct Claims {
  exp: usize,
  first_name: String,
}

impl Claims {
  fn new(kind: TokenKind, name: &str) -> Self {
    let now = OffsetDateTime::now_utc();
    let expiry = match kind {
      TokenKind::Access => (now.unix_timestamp() + THIRTY_MINUTES_AS_SECS) as usize,
      TokenKind::Refresh => (now.unix_timestamp() + THIRTY_DAYS_AS_SECS) as usize,
    };

    Self {
      exp: expiry,
      first_name: name.to_owned(),
    }
  }

  fn from_token(kind: TokenKind, token: &str) -> anyhow::Result<TokenData<Self>> {
    let validation = Validation::default();
    let secret = kind.to_secret_as_bytes();
    let secret = DecodingKey::from_secret(&secret);

    jsonwebtoken::decode(token, &secret, &validation).map_err(anyhow::Error::from)
  }

  fn into_token(&self, kind: TokenKind) -> anyhow::Result<String> {
    let header = Header::default();
    let secret = kind.to_secret_as_bytes();
    let secret = EncodingKey::from_secret(&secret);

    jsonwebtoken::encode(&header, self, &secret).map_err(anyhow::Error::from)
  }

  fn into_cookie<'l>(self, kind: TokenKind, value: String) -> Cookie<'l> {
    let name = kind.to_cookie_name();
    let base = Cookie::new(name, value);
    let expiry = OffsetDateTime::from_unix_timestamp(self.exp as i64)
      .unwrap_or_else(|_| OffsetDateTime::now_utc());

    Cookie::build(base)
      .http_only(true)
      .secure(true)
      .expires(expiry)
      .build()
  }
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
  let access_key = TokenKind::Access.to_cookie_name();
  if cookies.get(&access_key).is_some() {
    return Err(StatusCode::NO_CONTENT).map_err(HandlerError::from);
  }

  let user = get_user_by_name(&pool, &payload.first_name).await?;

  let access_claims = Claims::new(TokenKind::Access, &user.first_name);
  let access_token = access_claims.into_token(TokenKind::Access)?;

  let refresh_claims = Claims::new(TokenKind::Refresh, &user.first_name);
  let refresh_token = refresh_claims.into_token(TokenKind::Refresh)?;

  if !set_refresh_token_by_user(&pool, &refresh_token, &user.first_name).await {
    return Err(StatusCode::INTERNAL_SERVER_ERROR).map_err(HandlerError::from);
  }

  cookies.add(access_claims.into_cookie(TokenKind::Access, access_token));
  cookies.add(refresh_claims.into_cookie(TokenKind::Refresh, refresh_token));

  Ok(StatusCode::OK)
}

pub async fn protected(
  cookies: Cookies,
  State(pool): State<Arc<PgPool>>,
  mut request: Request,
  next: Next,
) -> HandlerResult<impl IntoResponse> {
  let refresh_key = TokenKind::Refresh.to_cookie_name();
  let access_key = TokenKind::Access.to_cookie_name();

  let refresh_token = cookies
    .get(&refresh_key)
    .map(|cookie| cookie.value().to_string())
    .ok_or(StatusCode::UNAUTHORIZED)?;

  let claims = Claims::from_token(TokenKind::Refresh, &refresh_token).map(|data| data.claims)?;

  if cookies.get(access_key).is_none() {
    if !refresh_token_exists(&pool, &refresh_token).await {
      return Err(StatusCode::UNAUTHORIZED).map_err(HandlerError::from);
    }

    let access_claims = Claims::new(TokenKind::Access, &claims.first_name);
    let access_token = access_claims.into_token(TokenKind::Access)?;

    let refresh_claims = Claims::new(TokenKind::Refresh, &claims.first_name);
    let refresh_token = refresh_claims.into_token(TokenKind::Refresh)?;

    if !update_refresh_token(&pool, &refresh_token, &refresh_token).await {
      return Err(StatusCode::INTERNAL_SERVER_ERROR).map_err(HandlerError::from);
    }

    cookies.add(access_claims.into_cookie(TokenKind::Access, access_token));
    cookies.add(refresh_claims.into_cookie(TokenKind::Refresh, refresh_token));
  }

  request.extensions_mut().insert(UserContext {
    first_name: claims.first_name,
  });

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

async fn set_refresh_token_by_user(pool: &PgPool, token: &str, name: &str) -> bool {
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
    .is_ok()
}

async fn refresh_token_exists(pool: &PgPool, token: &str) -> bool {
  let statement = r#"
    SELECT EXISTS (
      SELECT 1 FROM users
      WHERE refresh_token = $1
      AND refresh_token IS NOT NULL
      LIMIT 1
    )
  "#;

  sqlx::query_scalar(statement)
    .bind(token)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn update_refresh_token(pool: &PgPool, new_token: &str, old_token: &str) -> bool {
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
    .is_ok()
}

#[cfg(test)]
mod tests {
  use axum_test::TestResponse;
  use axum_test::TestServer;

  use crate::create_app;

  use super::*;

  const ACCESS_COOKIE: &str = "access_token";
  const REFRESH_COOKIE: &str = "refresh_token";

  async fn setup() -> TestServer {
    let app = create_app().await.unwrap();

    TestServer::builder().build(app).unwrap()
  }

  fn assert_cookies(response: &TestResponse, exists: bool) {
    let access_cookie = response.maybe_cookie(ACCESS_COOKIE);
    let refresh_cookie = response.maybe_cookie(REFRESH_COOKIE);

    assert_eq!(access_cookie.is_some(), exists);
    assert_eq!(refresh_cookie.is_some(), exists);
  }

  #[tokio::test]
  async fn it_should_return_failure_unauthenticated() {
    let server = setup().await;

    let response = server.get("/").await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn it_should_return_success_authenticated() {
    let server = setup().await;
    let payload = serde_json::json!({ "firstName": "Alice" });

    server.post("/login").json(&payload).save_cookies().await;
    let response = server.get("/").await;

    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn it_should_refresh_access_token() {
    let server = setup().await;
    let payload = serde_json::json!({ "firstName": "Alice" });

    let response = server.post("/login").json(&payload).await;
    let refresh_cookie = response.cookie(REFRESH_COOKIE);
    let response = server
      .get("/")
      .add_cookie(refresh_cookie)
      .save_cookies()
      .await;

    assert_cookies(&response, true);
    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn it_should_return_success_on_valid_body() {
    let server = setup().await;
    let payload = serde_json::json!({ "firstName": "Alice" });

    let response = server.post("/login").json(&payload).await;

    assert_cookies(&response, true);
    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn it_should_return_failure_on_invalid_body() {
    let server = setup().await;
    let payload = serde_json::json!({ "foo": "bar" });

    let response = server.post("/login").json(&payload).expect_failure().await;

    assert_cookies(&response, false);
    assert_eq!(response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
  }
}
