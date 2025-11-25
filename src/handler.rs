use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use axum::Extension;
use axum::Json;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use chrono::DateTime;
use chrono::Utc;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use jsonwebtoken::Validation;
use jsonwebtoken::decode;
use jsonwebtoken::encode;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use sqlx::query;
use sqlx::query_as;
use tower_cookies::Cookie;
use tower_cookies::Cookies;
use tower_cookies::cookie::time::OffsetDateTime;
use tracing::error;

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
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserContext {
  first_name: String,
}

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
    error!("[DATABASE] {err}");
    Self(StatusCode::INTERNAL_SERVER_ERROR)
  }
}

impl From<anyhow::Error> for HandlerError {
  fn from(err: anyhow::Error) -> Self {
    error!("[UNEXPECTED] {err}");
    Self(StatusCode::INTERNAL_SERVER_ERROR)
  }
}

enum Token {
  Access,
  Refresh,
}

impl Token {
  const ACCESS_COOKIE_NAME: &str = "access_token";
  const REFRESH_COOKIE_NAME: &str = "refresh_token";

  const fn into_secret(self) -> &'static str {
    match self {
      Self::Access => env!("ACCESS_SECRET"),
      Self::Refresh => env!("REFRESH_SECRET"),
    }
  }

  const fn lifetime(self) -> Duration {
    match self {
      Self::Access => Duration::from_secs(1_800),
      Self::Refresh => Duration::from_secs(2_592_000),
    }
  }
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
  exp: u64,
  user: UserContext,
}

impl Claims {
  fn new_pair(user_claims: UserContext) -> (Self, Self) {
    let now = SystemTime::now();
    let access_duration = Token::Access.lifetime();
    let refresh_duration = Token::Refresh.lifetime();

    let build_expiry = |duration: Duration| {
      now
        .add(duration)
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
    };

    let exp = build_expiry(access_duration);
    let user = user_claims.clone();
    let access_claims = Self { exp, user };

    let exp = build_expiry(refresh_duration);
    let user = user_claims;
    let refresh_claims = Self { exp, user };

    (access_claims, refresh_claims)
  }

  fn into_token(&self, token_type: Token) -> anyhow::Result<String> {
    let header = Header::default();
    let secret = token_type.into_secret().as_bytes();
    let secret = EncodingKey::from_secret(&secret);

    encode(&header, self, &secret).map_err(anyhow::Error::from)
  }

  fn from_token(token_type: Token, token_value: &str) -> anyhow::Result<Self> {
    let validation = Validation::default();
    let secret = token_type.into_secret().as_bytes();
    let secret = DecodingKey::from_secret(&secret);

    decode(token_value, &secret, &validation)
      .map(|data| data.claims)
      .map_err(anyhow::Error::from)
  }

  fn into_cookie<'l>(
    self,
    cookie_name: &'l str,
    token_value: String,
  ) -> anyhow::Result<Cookie<'l>> {
    let base = Cookie::new(cookie_name, token_value);
    let expiry = OffsetDateTime::from_unix_timestamp(self.exp as i64)?;
    let cookie = Cookie::build(base)
      .http_only(true)
      .secure(true)
      .expires(expiry)
      .build();

    Ok(cookie)
  }
}

pub async fn index(Extension(user): Extension<UserContext>) -> HandlerResult<impl IntoResponse> {
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
  Json(body): Json<LoginPayload>,
) -> HandlerResult<impl IntoResponse> {
  if cookies.get(Token::ACCESS_COOKIE_NAME).is_some() {
    return Err(StatusCode::NO_CONTENT).map_err(HandlerError::from);
  }

  let user = get_user_by_name(&pool, &body.first_name).await?;
  let first_name = user.first_name.clone();

  let user_context = UserContext { first_name };
  let (access_claims, refresh_claims) = Claims::new_pair(user_context);
  let access_token = access_claims.into_token(Token::Access)?;
  let refresh_token = refresh_claims.into_token(Token::Refresh)?;

  set_refresh_token_by_name(&pool, &refresh_token, &user.first_name).await?;

  let access_cookie = access_claims.into_cookie(Token::ACCESS_COOKIE_NAME, access_token)?;
  let refresh_cookie = refresh_claims.into_cookie(Token::REFRESH_COOKIE_NAME, refresh_token)?;
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
  let existing_refresh_token = cookies
    .get(Token::REFRESH_COOKIE_NAME)
    .ok_or(StatusCode::UNAUTHORIZED)?
    .value()
    .to_string();

  let existing_claims = Claims::from_token(Token::Refresh, &existing_refresh_token)?;
  if cookies.get(Token::ACCESS_COOKIE_NAME).is_none() {
    let first_name = existing_claims.user.first_name.clone();
    let user_context = UserContext { first_name };

    let (access_claims, refresh_claims) = Claims::new_pair(user_context);
    let access_token = access_claims.into_token(Token::Access)?;
    let refresh_token = refresh_claims.into_token(Token::Refresh)?;

    update_refresh_token(&pool, &refresh_token, &existing_refresh_token).await?;

    let access_cookie = access_claims.into_cookie(Token::ACCESS_COOKIE_NAME, access_token)?;
    let refresh_cookie = refresh_claims.into_cookie(Token::REFRESH_COOKIE_NAME, refresh_token)?;
    cookies.add(access_cookie);
    cookies.add(refresh_cookie);
  }

  request.extensions_mut().insert(existing_claims.user);
  Ok(next.run(request).await)
}

async fn get_user_by_name(pool: &PgPool, name: &str) -> HandlerResult<User> {
  let statement = r#"
    SELECT * FROM users
    WHERE first_name = $1
    LIMIT 1
  "#;

  query_as(statement)
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

  query(statement)
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

  query(statement)
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
  use serde_json::json;

  use crate::create_app;
  use crate::setup_logging;

  use super::*;

  static BEFORE_ALL: Once = Once::new();

  async fn setup() -> TestServer {
    BEFORE_ALL.call_once(setup_logging);

    let app = create_app().await.unwrap();
    TestServer::builder().build(app).unwrap()
  }

  fn valid_payload() -> Value {
    json!({ "firstName": "Alice" })
  }

  fn assert_cookies(response: &TestResponse, should_exist: bool) {
    let access_cookie = response.maybe_cookie(Token::ACCESS_COOKIE_NAME);
    let refresh_cookie = response.maybe_cookie(Token::REFRESH_COOKIE_NAME);

    assert_eq!(access_cookie.is_some(), should_exist);
    assert_eq!(refresh_cookie.is_some(), should_exist);
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
    let payload = json!({ "foo": "bar" });
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

    assert_eq!(response.text(), "Alice".to_string());
    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn expired_access_token_protected_route() {
    let server = setup().await;
    let payload = valid_payload();

    let response = server.post("/login").json(&payload).await;
    let refresh_cookie = response.cookie(Token::REFRESH_COOKIE_NAME);
    let response = server
      .get("/")
      .add_cookie(refresh_cookie)
      .save_cookies()
      .await;

    assert_cookies(&response, true);
    assert_eq!(response.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn missing_refresh_token_protected_route() {
    let server = setup().await;
    let payload = valid_payload();

    let response = server.post("/login").json(&payload).await;
    response.cookies().force_remove(Token::REFRESH_COOKIE_NAME);
    let response = server.get("/").expect_failure().await;

    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
  }
}
