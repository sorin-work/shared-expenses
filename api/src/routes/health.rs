use serde_json::{Value, json};

use axum::Json;

pub async fn check() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
