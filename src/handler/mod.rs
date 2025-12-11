use axum::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use serde::Deserialize;
use serde::Serialize;
use tracing::error;

pub mod auth;

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

#[derive(Clone, Serialize, Deserialize)]
pub struct UserState {
  id: i32,
}

pub async fn index(Extension(user): Extension<UserState>) -> HandlerResult<impl IntoResponse> {
  let user_id = user.id.to_string();
  Ok(user_id)
}
