use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Field name -> message
pub type FieldErrors = BTreeMap<String, String>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("some fields are invalid")]
    Validation(FieldErrors),
    #[error("{0}")]
    BadRequest(String),
    #[error("you need to sign in")]
    Unauthorized,
    #[error("wrong email or password")]
    InvalidCredentials,
    #[error("{0}")]
    Forbidden(&'static str),
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal(String),
}

impl AppError {
    pub fn field(field: &str, message: impl Into<String>) -> Self {
        Self::Validation(BTreeMap::from([(field.to_string(), message.into())]))
    }

    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed"),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::Database(_) | Self::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        match &self {
            Self::Database(err) => tracing::error!(error = %err, "database error"),
            Self::Internal(msg) => tracing::error!(error = %msg, "internal error"),
            _ => {}
        }

        let mut body = json!({ "code": code, "message": self.to_string() });
        if let Self::Validation(fields) = &self {
            body["fields"] = json!(fields);
        }
        (status, Json(json!({ "error": body }))).into_response()
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

impl From<PathRejection> for AppError {
    fn from(_: PathRejection) -> Self {
        Self::NotFound("resource")
    }
}

impl From<QueryRejection> for AppError {
    fn from(rejection: QueryRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

/// True if `err` is a unique constraint violation on `constraint`.
pub fn is_unique_violation(err: &sqlx::Error, constraint: &str) -> bool {
    match err {
        sqlx::Error::Database(db) => {
            db.is_unique_violation() && db.constraint() == Some(constraint)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn body(err: AppError) -> (StatusCode, serde_json::Value) {
        let response = err.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn validation_errors_list_the_fields() {
        let (status, json) = body(AppError::field("email", "is invalid")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json["error"]["code"], "validation_failed");
        assert_eq!(json["error"]["fields"]["email"], "is invalid");
    }

    #[tokio::test]
    async fn not_found_names_the_resource() {
        let (status, json) = body(AppError::NotFound("group")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["error"]["message"], "group not found");
    }

    #[tokio::test]
    async fn internal_errors_do_not_leak_details() {
        let (status, json) = body(AppError::Internal("disk on fire".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"]["message"], "internal error");
    }
}
