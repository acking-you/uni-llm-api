use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;

/// Make our own error that wraps `anyhow::Error`.
pub(crate) struct AppError(anyhow::Error);

/// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("Error: {:?}", self.0);
        (
            StatusCode::BAD_REQUEST,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

/// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
/// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
