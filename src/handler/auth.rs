use std::ops::Add;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

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
  fn pair(state: &UserState) -> (Self, Self) {
    let now = SystemTime::now();
    let acc_lifetime = Duration::from_secs(1_800); // 30m
    let ref_lifetime = Duration::from_secs(2_592_000); // 30d

    let build_exp = |lifetime: Duration| {
      now
        .add(lifetime)
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
    };

    let exp = build_exp(acc_lifetime);
    let user = state.clone();
    let acc_claims = Self { user, exp };

    let exp = build_exp(ref_lifetime);
    let user = state.clone();
    let ref_claims = Self { user, exp };

    (acc_claims, ref_claims)
  }

  fn into_cookie<'l>(&self, secret: &[u8], name: &'l str) -> anyhow::Result<Cookie<'l>> {
    let header = Header::default();
    let secret = EncodingKey::from_secret(&secret);
    let token = encode(&header, self, &secret).map_err(anyhow::Error::from)?;

    let base = Cookie::new(name, token);
    let now = OffsetDateTime::now_utc();
    let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64).unwrap_or(now);

    let cookie = Cookie::build(base)
      .http_only(true)
      .secure(true)
      .expires(exp)
      .build();

    Ok(cookie)
  }

  fn from_token(secret: &[u8], value: &str) -> anyhow::Result<Self> {
    let val = Validation::default();
    let secret = DecodingKey::from_secret(&secret);

    decode(value, &secret, &val)
      .map(|data| data.claims)
      .map_err(anyhow::Error::from)
  }
}

#[derive(Deserialize)]
pub struct LoginBody {
  id: i32,
}

pub async fn login(
  cookies: Cookies,
  State(state): State<AppState>,
  Json(body): Json<LoginBody>,
) -> HandlerResult<impl IntoResponse> {
  if cookies.get(ACC_COOKIE).is_some() || cookies.get(REF_COOKIE).is_some() {
    return Err(StatusCode::NO_CONTENT).map_err(HandlerError::from);
  }

  if !user_exists_by_id(&state.pool, body.id).await? {
    return Err(StatusCode::NOT_FOUND).map_err(HandlerError::from);
  }

  let user = UserState { id: body.id };
  let (acc_claims, ref_claims) = Claims::pair(&user);
  let secret = state.token_secret.as_bytes();

  let acc_cookie = acc_claims.into_cookie(secret, ACC_COOKIE)?;
  let ref_cookie = ref_claims.into_cookie(secret, REF_COOKIE)?;
  set_refresh_token_by_id(&state.pool, ref_cookie.value(), body.id).await?;

  cookies.add(acc_cookie);
  cookies.add(ref_cookie);

  Ok(StatusCode::OK)
}

pub async fn auth_middleware(
  cookies: Cookies,
  State(state): State<AppState>,
  mut req: Request,
  next: Next,
) -> HandlerResult<impl IntoResponse> {
  let user_token = cookies
    .get(REF_COOKIE)
    .ok_or(StatusCode::UNAUTHORIZED)?
    .value()
    .to_string();

  let secret = state.token_secret.as_bytes();
  let Claims { user, .. } = Claims::from_token(secret, &user_token)?;

  if cookies.get(ACC_COOKIE).is_none() {
    let user = UserState { id: user.id };
    let (acc_claims, ref_claims) = Claims::pair(&user);

    let acc_cookie = acc_claims.into_cookie(secret, ACC_COOKIE)?;
    let ref_cookie = ref_claims.into_cookie(secret, REF_COOKIE)?;
    set_refresh_token_by_id(&state.pool, ref_cookie.value(), user.id).await?;

    cookies.add(acc_cookie);
    cookies.add(ref_cookie);
  }

  req.extensions_mut().insert(user);
  Ok(next.run(req).await)
}

async fn user_exists_by_id(pool: &PgPool, id: i32) -> HandlerResult<bool> {
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

async fn set_refresh_token_by_id(pool: &PgPool, token: &str, id: i32) -> HandlerResult<bool> {
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

    let exp_body = "1".to_string();
    assert_eq!(res.text(), exp_body);
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
}
