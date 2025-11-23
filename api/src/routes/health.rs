use serde_json::{Value, json};

use crate::extract::Json;

pub async fn check() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
