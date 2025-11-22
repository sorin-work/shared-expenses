use std::collections::BTreeMap;

use axum::Json;
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

/// True if `err` is a unique constraint violation on `constraint`.
pub fn is_unique_violation(err: &sqlx::Error, constraint: &str) -> bool {
    match err {
        sqlx::Error::Database(db) => {
            db.is_unique_violation() && db.constraint() == Some(constraint)
        }
        _ => false,
    }
}
