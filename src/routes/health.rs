use axum::Json;
use serde_json::{Value, json};

/// `GET /healthz` — liveness probe.
pub async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
