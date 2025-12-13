use std::ops::Add;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use axum::Json;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use jsonwebtoken::DecodingKey;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use jsonwebtoken::Validation;
use jsonwebtoken::decode;
use jsonwebtoken::encode;
use serde::Deserialize;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::query;
use sqlx::query_scalar;
use tower_cookies::Cookie;
use tower_cookies::Cookies;
use tower_cookies::cookie::time::OffsetDateTime;

use crate::AppState;
use crate::handler::HandlerError;
use crate::handler::HandlerResult;
use crate::handler::UserState;

const ACC_COOKIE: &str = "acc";
const REF_COOKIE: &str = "ref";

#[derive(Serialize, Deserialize)]
pub struct Claims {
  exp: u64,
  user: UserState,
}

impl Claims {
  fn new_pair(user: &UserState) -> (Self, Self) {
    let now = SystemTime::now();

    let builder = |lifetime: Duration| -> Claims {
      let user = user.clone();
      let exp = now
        .add(lifetime)
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

      Claims { exp, user }
    };

    let acc_lifetime = Duration::from_secs(1_800); // 30m
    let ref_lifetime = Duration::from_secs(2_592_000); // 30d

    (builder(acc_lifetime), builder(ref_lifetime))
  }

  fn into_token(&self, secret: &[u8]) -> Result<String> {
    let header = Header::default();
    let secret = EncodingKey::from_secret(&secret);

    encode(&header, self, &secret).map_err(anyhow::Error::from)
  }

  fn from_token(secret: &[u8], value: &str) -> Result<Self> {
    let validator = Validation::default();
    let secret = DecodingKey::from_secret(&secret);

    decode(value, &secret, &validator)
      .map(|data| data.claims)
      .map_err(anyhow::Error::from)
  }

  fn set_cookie(&self, cookies: &mut Cookies, name: &'static str, value: String) {
    let base = Cookie::new(name, value);
    let now = OffsetDateTime::now_utc();
    let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64).unwrap_or(now);

    let cookie = Cookie::build(base)
      .http_only(true)
      .secure(true)
      .expires(exp)
      .build();

    cookies.add(cookie);
  }
}

pub async fn auth_middleware(
  mut cookies: Cookies,
  State(state): State<AppState>,
  mut req: Request,
  next: Next,
) -> HandlerResult<impl IntoResponse> {
  let token = cookies
    .get(REF_COOKIE)
    .ok_or(StatusCode::UNAUTHORIZED)?
    .value()
    .to_string();

  let secret = state.token_secret.as_bytes();
  let claims = Claims::from_token(secret, &token)?;

  if cookies.get(ACC_COOKIE).is_none() {
    let (acc_claims, ref_claims) = Claims::new_pair(&claims.user);

    let acc_token = acc_claims.into_token(secret)?;
    let ref_token = ref_claims.into_token(secret)?;

    set_token_by_id(&state.pool, &ref_token, claims.user.id).await?;
    acc_claims.set_cookie(&mut cookies, ACC_COOKIE, acc_token);
    ref_claims.set_cookie(&mut cookies, REF_COOKIE, ref_token);
  }

  req.extensions_mut().insert(claims.user);
  Ok(next.run(req).await)
}

#[derive(Deserialize)]
pub struct LoginBody {
  id: i32,
}

pub async fn login(
  mut cookies: Cookies,
  State(state): State<AppState>,
  Json(body): Json<LoginBody>,
) -> HandlerResult<impl IntoResponse> {
  if cookies.get(ACC_COOKIE).is_some() {
    return Err(StatusCode::NO_CONTENT).map_err(HandlerError::from);
  }

  if exists_by_id(&state.pool, body.id).await? {
    let secret = state.token_secret.as_bytes();

    let user = UserState::new(body.id);
    let (acc_claims, ref_claims) = Claims::new_pair(&user);

    let acc_token = acc_claims.into_token(secret)?;
    let ref_token = ref_claims.into_token(secret)?;

    set_token_by_id(&state.pool, &ref_token, body.id).await?;
    acc_claims.set_cookie(&mut cookies, ACC_COOKIE, acc_token);
    ref_claims.set_cookie(&mut cookies, REF_COOKIE, ref_token);
  } else {
    return Err(StatusCode::NOT_FOUND).map_err(HandlerError::from);
  }

  Ok(StatusCode::OK)
}

async fn exists_by_id(pool: &PgPool, id: i32) -> HandlerResult<bool> {
  let stmt = r#"
  SELECT EXISTS (
    SELECT 1 FROM users
    WHERE id = $1
  )
  "#;

  query_scalar(stmt)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(HandlerError::from)
}

async fn set_token_by_id(pool: &PgPool, token: &str, id: i32) -> HandlerResult<bool> {
  let stmt = r#"
  UPDATE users
  SET refresh_token = $1
  WHERE id = $2
  "#;

  query(stmt)
    .bind(token)
    .bind(id)
    .execute(pool)
    .await
    .map_err(HandlerError::from)
    .map(|res| res.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
  use std::sync::Once;

  use axum_test::TestResponse;
  use axum_test::TestServer;
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

  fn assert_cookies(res: &TestResponse, exists: bool) {
    let acc_cookie = res.maybe_cookie(ACC_COOKIE);
    let ref_cookie = res.maybe_cookie(REF_COOKIE);

    assert_eq!(acc_cookie.is_some(), exists);
    assert_eq!(ref_cookie.is_some(), exists);
  }

  #[tokio::test]
  async fn login_valid_body() {
    let server = setup().await;
    let body = json!({ "id": 1 });
    let res = server.post("/login").json(&body).await;

    assert_cookies(&res, true);
    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn login_invalid_body() {
    let server = setup().await;
    let body = json!({ "foo": "bar" });
    let res = server.post("/login").json(&body).expect_failure().await;

    assert_cookies(&res, false);
    assert_eq!(res.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
  }

  #[tokio::test]
  async fn login_unknown_body() {
    let server = setup().await;
    let body = json!({ "id": 9999 });
    let res = server.post("/login").json(&body).expect_failure().await;

    assert_cookies(&res, false);
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
  }

  #[tokio::test]
  async fn protected_authenticated() {
    let server = setup().await;
    let body = json!({ "id": 1 });

    server.post("/login").json(&body).save_cookies().await;
    let res = server.get("/").await;

    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn protected_unauthenticated() {
    let server = setup().await;
    let res = server.get("/").await;

    assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn protected_expired_acc_token() {
    let server = setup().await;
    let body = json!({ "id": 1 });

    let res = server.post("/login").json(&body).await;
    let ref_cookie = res.cookie(REF_COOKIE);
    let res = server.get("/").add_cookie(ref_cookie).save_cookies().await;

    assert_cookies(&res, true);
    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn protected_expired_ref_token() {
    let server = setup().await;
    let body = json!({ "id": 1 });

    let res = server.post("/login").json(&body).await;
    res.cookies().force_remove(REF_COOKIE);
    let res = server.get("/").expect_failure().await;

    assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn protected_invalid_ref_token() {
    let server = setup().await;
    let body = json!({ "id": 1 });

    let res = server.post("/login").json(&body).await;

    let mut ref_cookie = res.cookie(REF_COOKIE);
    let cloned = ref_cookie.clone();
    let malformed = cloned.value();
    ref_cookie.set_value(&malformed[10..]);

    let res = server
      .get("/")
      .add_cookie(ref_cookie)
      .expect_failure()
      .await;

    assert_eq!(res.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
  }
}
