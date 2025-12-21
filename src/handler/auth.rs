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
use tower_cookies::Cookie;
use tower_cookies::Cookies;
use tower_cookies::cookie::time::OffsetDateTime;

use crate::AppState;
use crate::handler::ACC_NAME;
use crate::handler::HandlerError;
use crate::handler::HandlerResult;
use crate::handler::REF_NAME;
use crate::handler::UserExtension;

#[derive(Serialize, Deserialize)]
pub struct Claims {
  exp: u64,
  user: UserExtension,
}

impl Claims {
  /// New access and refresh claims using the passed [`UserExtension`] and
  /// hard-coded lifetimes.
  fn new_pair(user: UserExtension) -> (Self, Self) {
    let now = SystemTime::now();
    let acc_life = Duration::from_secs(1_800); // 30 minutes
    let ref_life = Duration::from_secs(2_592_000); // 30 days

    let builder = |lifespan: Duration| {
      // NOTE: First call will clone unnecessarily but it's small enough.
      let user = user.clone();
      let exp = now
        .add(lifespan)
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO) // Immediate invalidation.
        .as_secs();

      Self { user, exp }
    };

    (builder(acc_life), builder(ref_life))
  }

  /// Create a JWT from the [`Claims`].
  ///
  /// NOTE: Default [`Header`] is used.
  fn into_token(&self, secret: &[u8]) -> Result<String> {
    let head = Header::default();
    let secret = EncodingKey::from_secret(secret);

    encode(&head, self, &secret).map_err(anyhow::Error::from)
  }

  /// Create a [`Claims`] from a given JWT string.
  ///
  /// NOTE: Default [`Validation`] is used.
  fn from_token(secret: &[u8], token: &str) -> Result<Self> {
    let validator = Validation::default();
    let secret = DecodingKey::from_secret(secret);

    decode(token, &secret, &validator)
      .map(|data| data.claims)
      .map_err(anyhow::Error::from)
  }

  /// Build and set a new [`Cookie`] with the given name, value and [`Claims`]
  /// expiry. Align the expiries so both cookie and token expire at the same
  /// time.
  fn into_cookie(self, cookies: &mut Cookies, name: &'static str, value: String) {
    let base = Cookie::new(name, value);
    let now = OffsetDateTime::now_utc();
    let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64).unwrap_or(now); // Immediate invalidation.

    let cookie = Cookie::build(base)
      .http_only(true)
      .secure(true)
      .expires(exp)
      .build();

    cookies.add(cookie);
  }
}

#[derive(Deserialize)]
pub struct LoginBody {
  id: i32,
}

/// Assuming missing access token, create a new pair of tokens and cookies. The
/// refresh token is updated/set against the user.
///
/// TODO: There is no actual payload validation e.g. email, password.
pub async fn login(
  mut cookies: Cookies,
  State(state): State<AppState>,
  Json(body): Json<LoginBody>,
) -> HandlerResult<impl IntoResponse> {
  // NOTE: Only the access token needs to be checked as the middleware should
  // always build an access token if authorised.
  if cookies.get(ACC_NAME).is_some() {
    return Err(StatusCode::NO_CONTENT).map_err(HandlerError::from);
  }

  // Use the provided user data to construct new tokens and cookies.
  let user = UserExtension { id: body.id };
  let secret = state.secret.as_bytes();

  let (acc_claims, ref_claims) = Claims::new_pair(user);
  let acc_token = acc_claims.into_token(&secret)?;
  let ref_token = ref_claims.into_token(&secret)?;

  set_refresh_token(&state.pool, &ref_token, body.id).await?;

  acc_claims.into_cookie(&mut cookies, ACC_NAME, acc_token);
  ref_claims.into_cookie(&mut cookies, REF_NAME, ref_token);

  Ok(StatusCode::OK)
}

pub async fn auth(
  mut cookies: Cookies,
  State(state): State<AppState>,
  mut req: Request,
  next: Next,
) -> HandlerResult<impl IntoResponse> {
  // Take ownership of the refresh token or return if missing.
  let token = match cookies.get(REF_NAME) {
    Some(cookie) => cookie.value().to_string(),
    None => return Err(StatusCode::UNAUTHORIZED).map_err(HandlerError::from),
  };

  let secret = state.secret.as_bytes();
  let claims = Claims::from_token(&secret, &token)?;

  // User has the refresh token, access token has expired.
  if cookies.get(ACC_NAME).is_none() {
    // Use the data from the decoded claims to construct new tokens and cookies.
    let user = UserExtension { id: claims.user.id };

    let (acc_claims, ref_claims) = Claims::new_pair(user);
    let acc_token = acc_claims.into_token(&secret)?;
    let ref_token = ref_claims.into_token(&secret)?;

    set_refresh_token(&state.pool, &ref_token, claims.user.id).await?;

    acc_claims.into_cookie(&mut cookies, ACC_NAME, acc_token);
    ref_claims.into_cookie(&mut cookies, REF_NAME, ref_token);
  }

  // NOTE: Whether the tokens have been regenerated or not, use the decoded
  // claims data as it remains valid.
  req.extensions_mut().insert(claims.user);
  Ok(next.run(req).await)
}

/// Assign/update the refresh token by a user's ID. Returns unit if at least one
/// row was affected.
async fn set_refresh_token(pool: &PgPool, token: &str, id: i32) -> HandlerResult<()> {
  let stmt = r#"
    UPDATE users
    SET refresh_token = $1
    WHERE id = $2
  "#;

  let result = query(stmt)
    .bind(token)
    .bind(id)
    .execute(pool)
    .await
    .map_err(HandlerError::from)?;

  // Query was successful but no ID match was found.
  if result.rows_affected() <= 0 {
    return Err(StatusCode::NOT_FOUND).map_err(HandlerError::from);
  }

  Ok(())
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

  const LOGIN_ROUTE: &str = "/login";
  const PROTECTED_ROUTE: &str = "/";

  async fn setup() -> TestServer {
    BEFORE_ALL.call_once(setup_logging);

    let app = create_app().await.unwrap();
    TestServer::builder().build(app).unwrap()
  }

  /// Helper to assert the existence/non-existence of both the access and
  /// refresh cookies.
  ///
  /// NOTE: No distinction between cookie existence.
  fn assert_cookies(res: &TestResponse, exists: bool) {
    let acc_cookie = res.maybe_cookie(ACC_NAME);
    let ref_cookie = res.maybe_cookie(REF_NAME);

    assert_eq!(acc_cookie.is_some(), exists);
    assert_eq!(ref_cookie.is_some(), exists);
  }

  #[tokio::test]
  async fn login_valid_body() {
    let server = setup().await;
    let body = json!({ "id": 1 }); // Known user.

    let res = server.post(LOGIN_ROUTE).json(&body).await;

    assert_cookies(&res, true);
    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn login_invalid_body() {
    let server = setup().await;
    let body = json!({ "foo": "bar" }); // Unknown key.

    let res = server.post(LOGIN_ROUTE).json(&body).await;

    assert_cookies(&res, false);
    assert_eq!(res.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
  }

  #[tokio::test]
  async fn login_unknown_body() {
    let server = setup().await;
    let body = json!({ "id": 9999 }); // Unknown user.

    let res = server.post(LOGIN_ROUTE).json(&body).await;

    assert_cookies(&res, false);
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
  }

  #[tokio::test]
  async fn protected_authenticated() {
    let server = setup().await;
    let body = json!({ "id": 1 });

    // Request a protected route with the returned cookies from logging in.
    server.post(LOGIN_ROUTE).json(&body).save_cookies().await;
    let res = server.get(PROTECTED_ROUTE).await;

    assert_eq!(res.text(), "1".to_string());
    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn protected_unauthenticated() {
    let server = setup().await;

    // Request a protected route with no cookies.
    let res = server.get(PROTECTED_ROUTE).await;

    assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);
  }

  #[tokio::test]
  async fn protected_missing_access_cookie() {
    let server = setup().await;
    let body = json!({ "id": 1 });

    // Only pass the refresh cookie to the protected route. The access token is
    // regenerated using the decoded refresh token.
    let res = server.post(LOGIN_ROUTE).json(&body).await;
    let cookie = res.cookie(REF_NAME);
    let res = server.get(PROTECTED_ROUTE).add_cookie(cookie).await;

    assert_eq!(res.status_code(), StatusCode::OK);
  }

  #[tokio::test]
  async fn protected_missing_refresh_cookie() {
    let server = setup().await;
    let body = json!({ "id": 1 });

    // Only pass the access cookie to the protected route. Without the refresh
    // cookie, we can't decode the user's claims.
    let res = server.post(LOGIN_ROUTE).json(&body).await;
    let cookie = res.cookie(ACC_NAME);
    let res = server.get(PROTECTED_ROUTE).add_cookie(cookie).await;

    assert_eq!(res.status_code(), StatusCode::UNAUTHORIZED);
  }
}
