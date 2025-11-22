use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthUser, password};
use crate::error::{AppError, is_unique_violation};
use crate::state::AppState;
use crate::validate::Validator;
use axum::Json;

pub const MIN_PASSWORD_LEN: usize = 8;
pub const MAX_PASSWORD_LEN: usize = 128;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me))
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Session {
    pub token: String,
    pub user: User,
}

#[derive(Deserialize)]
pub struct SignupInput {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginInput {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

fn check_password(v: &mut Validator, field: &str, password: &str) {
    let len = password.chars().count();
    v.check(
        len >= MIN_PASSWORD_LEN,
        field,
        "must be at least 8 characters",
    );
    v.check(
        len <= MAX_PASSWORD_LEN,
        field,
        "must be at most 128 characters",
    );
}

/// Inserts a user. Shared with the demo seed.
pub async fn create_user(
    pool: &sqlx::PgPool,
    email: &str,
    name: &str,
    password_hash: &str,
) -> Result<User, AppError> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, name, password_hash) VALUES ($1, $2, $3, $4)
         RETURNING id, email, name, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(name)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .map_err(|err| {
        if is_unique_violation(&err, "users_email_key") {
            AppError::field("email", "is already registered")
        } else {
            err.into()
        }
    })
}

async fn signup(
    State(state): State<AppState>,
    Json(input): Json<SignupInput>,
) -> Result<(StatusCode, Json<Session>), AppError> {
    let mut v = Validator::new();
    let email = v.email("email", &input.email).to_lowercase();
    let name = v.text("name", &input.name, 60);
    check_password(&mut v, "password", &input.password);
    v.finish()?;

    let hash = password::hash_async(input.password).await?;
    let user = create_user(&state.pool, &email, &name, &hash).await?;

    let token = state.tokens.issue(user.id)?;
    Ok((StatusCode::CREATED, Json(Session { token, user })))
}

#[derive(sqlx::FromRow)]
struct UserWithHash {
    #[sqlx(flatten)]
    user: User,
    password_hash: String,
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Json<Session>, AppError> {
    let mut v = Validator::new();
    v.check(!input.email.trim().is_empty(), "email", "is required");
    v.check(!input.password.is_empty(), "password", "is required");
    v.finish()?;

    let found = sqlx::query_as::<_, UserWithHash>(
        "SELECT id, email, name, created_at, password_hash FROM users
         WHERE lower(email) = lower($1)",
    )
    .bind(input.email.trim())
    .fetch_optional(&state.pool)
    .await?;

    let Some(found) = found else {
        return Err(AppError::InvalidCredentials);
    };
    if !password::verify_async(input.password, found.password_hash).await? {
        return Err(AppError::InvalidCredentials);
    }
    let token = state.tokens.issue(found.user.id)?;
    Ok(Json(Session {
        token,
        user: found.user,
    }))
}

pub async fn load_user(pool: &sqlx::PgPool, id: Uuid) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT id, email, name, created_at FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        // A valid token for a user that no longer exists.
        .ok_or(AppError::Unauthorized)
}

async fn me(State(state): State<AppState>, auth: AuthUser) -> Result<Json<User>, AppError> {
    Ok(Json(load_user(&state.pool, auth.id).await?))
}
