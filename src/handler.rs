use std::ops::Add;
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
use sqlx::query;
use sqlx::query_scalar;
use tower_cookies::Cookie;
use tower_cookies::Cookies;
use tower_cookies::cookie::time::OffsetDateTime;
use tracing::error;

use crate::AppState;

const ACCESS_COOKIE: &str = "access_token";
const REFRESH_COOKIE: &str = "refresh_token";

pub struct HandlerError(StatusCode);
type HandlerResult<T> = Result<T, HandlerError>;

impl IntoResponse for HandlerError
{
  fn into_response(self) -> Response
  {
    self.0.into_response()
  }
}

impl From<StatusCode> for HandlerError
{
  fn from(code: StatusCode) -> Self
  {
    Self(code)
  }
}

impl From<sqlx::Error> for HandlerError
{
  fn from(err: sqlx::Error) -> Self
  {
    error!("[DATABASE] {err}");
    Self(StatusCode::INTERNAL_SERVER_ERROR)
  }
}

impl From<anyhow::Error> for HandlerError
{
  fn from(err: anyhow::Error) -> Self
  {
    error!("[UNEXPECTED] {err}");
    Self(StatusCode::INTERNAL_SERVER_ERROR)
  }
}

enum Token
{
  Access,
  Refresh,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserExtension
{
  id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Claims
{
  exp: u64,
  user: UserExtension,
}

impl Claims
{
  fn new_pair(user_ext: UserExtension) -> (Self, Self)
  {
    let now = SystemTime::now();
    let acc_lifetime = Duration::from_secs(1_800);
    let ref_lifetime = Duration::from_secs(2_592_000);

    let build_exp = |lifetime: Duration| {
      now
        .add(lifetime)
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
    };

    let exp = build_exp(acc_lifetime);
    let user = user_ext.clone();
    let acc_claims = Self { user, exp };

    let exp = build_exp(ref_lifetime);
    let user = user_ext;
    let ref_claims = Self { user, exp };

    (acc_claims, ref_claims)
  }

  fn into_token(&self, state: &AppState, token: Token) -> anyhow::Result<String>
  {
    let secret = match token
    {
      Token::Access => state.acc_secret.as_bytes(),
      Token::Refresh => state.ref_secret.as_bytes(),
    };

    let head = Header::default();
    let secret = EncodingKey::from_secret(&secret);

    encode(&head, self, &secret).map_err(anyhow::Error::from)
  }

  fn from_token(state: &AppState, token: Token, value: &str) -> anyhow::Result<Self>
  {
    let secret = match token
    {
      Token::Access => state.acc_secret.as_bytes(),
      Token::Refresh => state.ref_secret.as_bytes(),
    };

    let val = Validation::default();
    let secret = DecodingKey::from_secret(&secret);

    decode(value, &secret, &val)
      .map(|data| data.claims)
      .map_err(anyhow::Error::from)
  }

  fn into_cookie<'l>(self, name: &'static str, value: String) -> Cookie<'l>
  {
    let base = Cookie::new(name, value);
    let now = OffsetDateTime::now_utc();
    let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64).unwrap_or(now);

    Cookie::build(base)
      .http_only(true)
      .secure(true)
      .expires(exp)
      .build()
  }
}

pub async fn index(Extension(user): Extension<UserExtension>) -> HandlerResult<impl IntoResponse>
{
  let id_str = user.id.to_string();
  Ok(id_str)
}

#[derive(Deserialize)]
pub struct LoginBody
{
  id: i32,
}

pub async fn login(
  cookies: Cookies,
  State(state): State<AppState>,
  Json(body): Json<LoginBody>,
) -> HandlerResult<impl IntoResponse>
{
  if cookies.get(ACCESS_COOKIE).is_some()
  {
    return Err(StatusCode::NO_CONTENT).map_err(HandlerError::from);
  }

  if !user_by_id_exists(&state.pool, body.id).await?
  {
    return Err(StatusCode::NOT_FOUND).map_err(HandlerError::from);
  }

  let user_ext = UserExtension { id: body.id };
  let (acc_claims, ref_claims) = Claims::new_pair(user_ext);
  let acc_token = acc_claims.into_token(&state, Token::Access)?;
  let ref_token = ref_claims.into_token(&state, Token::Refresh)?;

  set_refresh_token_by_id(&state.pool, &ref_token, body.id).await?;

  let acc_cookie = acc_claims.into_cookie(ACCESS_COOKIE, acc_token);
  let ref_cookie = ref_claims.into_cookie(REFRESH_COOKIE, ref_token);
  cookies.add(acc_cookie);
  cookies.add(ref_cookie);

  Ok(StatusCode::OK)
}

pub async fn auth(
  cookies: Cookies,
  State(state): State<AppState>,
  mut req: Request,
  next: Next,
) -> HandlerResult<impl IntoResponse>
{
  let curr_ref_token = cookies
    .get(REFRESH_COOKIE)
    .ok_or(StatusCode::UNAUTHORIZED)?
    .value()
    .to_string();

  let curr_ref_claims = Claims::from_token(&state, Token::Refresh, &curr_ref_token)?;
  if cookies.get(ACCESS_COOKIE).is_none()
  {
    let id = curr_ref_claims.user.id;
    let user_ext = UserExtension { id };

    let (acc_claims, ref_claims) = Claims::new_pair(user_ext);
    let acc_token = acc_claims.into_token(&state, Token::Access)?;
    let ref_token = ref_claims.into_token(&state, Token::Refresh)?;

    update_refresh_token(&state.pool, &ref_token, &curr_ref_token).await?;

    let acc_cookie = acc_claims.into_cookie(ACCESS_COOKIE, acc_token);
    let ref_cookie = ref_claims.into_cookie(REFRESH_COOKIE, ref_token);
    cookies.add(acc_cookie);
    cookies.add(ref_cookie);
  }

  req.extensions_mut().insert(curr_ref_claims.user);
  Ok(next.run(req).await)
}

async fn user_by_id_exists(pool: &PgPool, id: i32) -> HandlerResult<bool>
{
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

async fn set_refresh_token_by_id(
  pool: &PgPool,
  token: &str,
  id: i32,
) -> HandlerResult<PgQueryResult>
{
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
}

async fn update_refresh_token(
  pool: &PgPool,
  new_token: &str,
  old_token: &str,
) -> HandlerResult<PgQueryResult>
{
  let stmt = r#"
    UPDATE users
    SET refresh_token = $1
    WHERE refresh_token = $2
  "#;

  query(stmt)
    .bind(new_token)
    .bind(old_token)
    .execute(pool)
    .await
    .map_err(HandlerError::from)
}

#[cfg(test)]
mod tests
{
  use std::sync::Once;

  use axum_test::TestResponse;
  use axum_test::TestServer;
  use serde_json::json;

  use crate::create_app;
  use crate::setup_logging;

  use super::*;

  static BEFORE_ALL: Once = Once::new();

  async fn setup() -> TestServer
  {
    BEFORE_ALL.call_once(setup_logging);

    let app = create_app().await.unwrap();
    TestServer::builder().build(app).unwrap()
  }

  fn assert_cookies(res: &TestResponse, exists: bool)
  {
    let acc_cookie = res.maybe_cookie(ACCESS_COOKIE);
    let ref_cookie = res.maybe_cookie(REFRESH_COOKIE);

    assert_eq!(acc_cookie.is_some(), exists);
    assert_eq!(ref_cookie.is_some(), exists);
  }

  #[tokio::test]
  async fn login_valid_body()
  {
    let srvr = setup().await;
    let body = json!({ "id": 1 });
    let res = srvr.post("/login").json(&body).await;

    assert_cookies(&res, true);
    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn login_invalid_body()
  {
    let srvr = setup().await;
    let body = json!({ "foo": "bar" });
    let res = srvr.post("/login").json(&body).expect_failure().await;

    assert_cookies(&res, false);
    assert_eq!(res.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
  }

  #[tokio::test]
  async fn login_unknown_body()
  {
    let srvr = setup().await;
    let body = json!({ "id": 9999 });
    let res = srvr.post("/login").json(&body).expect_failure().await;

    assert_cookies(&res, false);
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
  }

  #[tokio::test]
  async fn protected_authenticated()
  {
    let srvr = setup().await;
    let body = json!({ "id": 1 });

    srvr.post("/login").json(&body).save_cookies().await;
    let res = srvr.get("/").await;

    let exp_body = "1".to_string();
    assert_eq!(res.text(), exp_body);
    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn protected_unauthenticated()
  {
    let srvr = setup().await;
    let res = srvr.get("/").await;

    assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn protected_expired_acc_token()
  {
    let srvr = setup().await;
    let body = json!({ "id": 1 });

    let res = srvr.post("/login").json(&body).await;
    let ref_cookie = res.cookie(REFRESH_COOKIE);
    let res = srvr.get("/").add_cookie(ref_cookie).save_cookies().await;

    assert_cookies(&res, true);
    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn protected_expired_ref_token()
  {
    let srvr = setup().await;
    let body = json!({ "id": 1 });

    let res = srvr.post("/login").json(&body).await;
    res.cookies().force_remove(REFRESH_COOKIE);
    let res = srvr.get("/").expect_failure().await;

    assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);
  }
}
