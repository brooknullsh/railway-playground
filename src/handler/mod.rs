use axum::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use serde::Deserialize;
use serde::Serialize;
use tracing::error;

pub mod auth;

const ACC_NAME: &str = "acc";
const REF_NAME: &str = "ref";

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

/// User data shared between requests. Set within authentication middleware.
#[derive(Clone, Serialize, Deserialize)]
pub struct UserExtension {
  id: i32,
}

/// Protected handler requiring authenticated request.
pub async fn index(Extension(user): Extension<UserExtension>) -> HandlerResult<impl IntoResponse> {
  let id_str = user.id.to_string();
  Ok(id_str)
}
