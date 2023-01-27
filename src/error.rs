#[derive(Debug)]
pub enum Error {
    Unauthorized,
    BadRequest(String),
    InternalServer(String),
}
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            Error::BadRequest(error) => (StatusCode::BAD_REQUEST, error),
            Error::InternalServer(error) => {
                tracing::error!("internal server error: {}", error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
