use std::sync::LazyLock;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthUser, password};
use crate::error::{AppError, is_unique_violation};
use crate::extract::Json;
use crate::state::AppState;
use crate::validate::Validator;

pub const MIN_PASSWORD_LEN: usize = 8;
pub const MAX_PASSWORD_LEN: usize = 128;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/me", get(me).patch(update_me))
        .route("/password", post(change_password))
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

#[derive(Deserialize)]
pub struct ProfileInput {
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize)]
pub struct PasswordInput {
    #[serde(default)]
    pub current_password: String,
    #[serde(default)]
    pub new_password: String,
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

/// Checked against when the email is unknown, so that a login for an
/// unknown email takes as long as one with a wrong password.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| password::hash("not the password").expect("hashing works"));

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

    let stored = found
        .as_ref()
        .map_or(DUMMY_HASH.as_str(), |f| f.password_hash.as_str())
        .to_string();
    let matches = password::verify_async(input.password, stored).await?;

    match found {
        Some(found) if matches => {
            let token = state.tokens.issue(found.user.id)?;
            Ok(Json(Session {
                token,
                user: found.user,
            }))
        }
        _ => Err(AppError::InvalidCredentials),
    }
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

async fn update_me(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<ProfileInput>,
) -> Result<Json<User>, AppError> {
    let mut v = Validator::new();
    let name = v.text("name", &input.name, 60);
    v.finish()?;

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET name = $2 WHERE id = $1 RETURNING id, email, name, created_at",
    )
    .bind(auth.id)
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    Ok(Json(user))
}

async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<PasswordInput>,
) -> Result<StatusCode, AppError> {
    let mut v = Validator::new();
    v.check(
        !input.current_password.is_empty(),
        "current_password",
        "is required",
    );
    check_password(&mut v, "new_password", &input.new_password);
    v.finish()?;

    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(auth.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !password::verify_async(input.current_password, stored).await? {
        return Err(AppError::field("current_password", "is not correct"));
    }

    let hash = password::hash_async(input.new_password).await?;
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(auth.id)
        .bind(hash)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
