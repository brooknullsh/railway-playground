use std::{env, sync::Arc};

use axum::{
  Extension, Json,
  extract::{Request, State},
  http::StatusCode,
  middleware::Next,
  response::IntoResponse,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, prelude::FromRow, types::chrono};
use tower_cookies::{Cookie, Cookies, cookie::time::OffsetDateTime};

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

  fn to_secret_as_bytes(self) -> anyhow::Result<Vec<u8>> {
    let key = match self {
      TokenKind::Access => "JWT_ACCESS_SECRET",
      TokenKind::Refresh => "JWT_REFRESH_SECRET",
    };

    let secret = env::var(key).inspect_err(|err| tracing::error!("{err}"))?;
    Ok(secret.into_bytes())
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

pub async fn index(Extension(user): Extension<UserContext>) -> impl IntoResponse {
  (StatusCode::OK, user.first_name).into_response()
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
      TokenKind::Access => (now.unix_timestamp() + 1_800) as usize,
      TokenKind::Refresh => (now.unix_timestamp() + 2_592_000) as usize,
    };

    Self {
      exp: expiry,
      first_name: name.to_owned(),
    }
  }

  fn from_token(kind: TokenKind, token: &str) -> anyhow::Result<TokenData<Self>> {
    let validation = Validation::default();
    let secret = kind.to_secret_as_bytes()?;
    let secret = DecodingKey::from_secret(&secret);

    jsonwebtoken::decode(token, &secret, &validation)
      .inspect_err(|err| tracing::error!("{err}"))
      .map_err(anyhow::Error::from)
  }

  fn into_token(&self, kind: TokenKind) -> anyhow::Result<String> {
    let header = Header::default();
    let secret = kind.to_secret_as_bytes()?;
    let secret = EncodingKey::from_secret(&secret);

    jsonwebtoken::encode(&header, self, &secret)
      .inspect_err(|err| tracing::error!("{err}"))
      .map_err(anyhow::Error::from)
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
) -> impl IntoResponse {
  let access_key = TokenKind::Access.to_cookie_name();
  if cookies.get(&access_key).is_some() {
    return StatusCode::NO_CONTENT.into_response();
  }

  let Ok(user) = get_user_by_name(&pool, &payload.first_name).await else {
    return StatusCode::NOT_FOUND.into_response();
  };

  let access_claims = Claims::new(TokenKind::Access, &user.first_name);
  let Ok(access_token) = access_claims.into_token(TokenKind::Access) else {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  };

  let refresh_claims = Claims::new(TokenKind::Refresh, &user.first_name);
  let Ok(refresh_token) = refresh_claims.into_token(TokenKind::Refresh) else {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  };

  if !set_refresh_token_by_user(&pool, &refresh_token, &user.first_name).await {
    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
  }

  cookies.add(access_claims.into_cookie(TokenKind::Access, access_token));
  cookies.add(refresh_claims.into_cookie(TokenKind::Refresh, refresh_token));

  StatusCode::OK.into_response()
}

pub async fn protected(
  cookies: Cookies,
  State(pool): State<Arc<PgPool>>,
  mut request: Request,
  next: Next,
) -> impl IntoResponse {
  let refresh_key = TokenKind::Refresh.to_cookie_name();
  let access_key = TokenKind::Access.to_cookie_name();

  let refresh_token = match cookies.get(&refresh_key) {
    Some(cookie) => cookie.value().to_string(),
    None => return StatusCode::UNAUTHORIZED.into_response(),
  };

  let refresh_claims = match Claims::from_token(TokenKind::Refresh, &refresh_token) {
    Ok(token_data) => token_data.claims,
    Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
  };

  if cookies.get(access_key).is_none() {
    if !refresh_token_exists(&pool, &refresh_token).await {
      return StatusCode::UNAUTHORIZED.into_response();
    }

    let access_claims = Claims::new(TokenKind::Access, &refresh_claims.first_name);
    let Ok(access_token) = access_claims.into_token(TokenKind::Access) else {
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let refresh_claims = Claims::new(TokenKind::Refresh, &refresh_claims.first_name);
    let Ok(refresh_token) = refresh_claims.into_token(TokenKind::Refresh) else {
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if !update_refresh_token(&pool, &refresh_token, &refresh_token).await {
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    cookies.add(access_claims.into_cookie(TokenKind::Access, access_token));
    cookies.add(refresh_claims.into_cookie(TokenKind::Refresh, refresh_token));
  }

  request.extensions_mut().insert(UserContext {
    first_name: refresh_claims.first_name,
  });

  next.run(request).await
}

async fn get_user_by_name(pool: &PgPool, name: &str) -> anyhow::Result<User> {
  let statement = r#"
    SELECT * FROM users
    WHERE first_name = $1
    LIMIT 1
  "#;

  sqlx::query_as(statement)
    .bind(name)
    .fetch_one(pool)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(anyhow::Error::from)
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
    .inspect_err(|err| tracing::error!("{err}"))
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
    .inspect_err(|err| tracing::error!("{err}"))
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
    .inspect_err(|err| tracing::error!("{err}"))
    .is_ok()
}
