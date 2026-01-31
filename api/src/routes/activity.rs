use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::access;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::extract::{Json, Path, Query};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct FeedQuery {
    /// Only entries older than this, for loading the next page.
    pub before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Entry {
    pub id: Uuid,
    pub kind: String,
    pub payload: serde_json::Value,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Feed {
    pub items: Vec<Entry>,
    /// Pass as `before` to get the next page; absent on the last page.
    pub next_before: Option<DateTime<Utc>>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<Feed>, AppError> {
    access::membership(&state.pool, group_id, auth.id).await?;
    let limit = query.limit.unwrap_or(30).clamp(1, 100);

    let items = sqlx::query_as::<_, Entry>(
        "SELECT a.id, a.kind, a.payload, a.actor_id, u.name AS actor_name, a.created_at
           FROM activity a
           LEFT JOIN users u ON u.id = a.actor_id
          WHERE a.group_id = $1 AND ($2::timestamptz IS NULL OR a.created_at < $2)
          ORDER BY a.created_at DESC, a.id DESC
          LIMIT $3",
    )
    .bind(group_id)
    .bind(query.before)
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    let next_before = (items.len() as i64 == limit)
        .then(|| items.last().map(|e| e.created_at))
        .flatten();
    Ok(Json(Feed { items, next_before }))
}
