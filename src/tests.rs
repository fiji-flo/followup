//! Integration tests exercising the router via `tower::ServiceExt::oneshot`.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;
use tower_sessions_sqlx_store::SqliteStore;
use uuid::Uuid;
use webauthn_rs::prelude::{Url, WebauthnBuilder};

use crate::routes::build_router;
use crate::state::AppState;

async fn test_app() -> Router {
    let path = std::env::temp_dir().join(format!("fu-test-{}.db", Uuid::new_v4()));
    let url = format!("sqlite://{}?mode=rwc", path.display());

    let pool = crate::db::init_pool(&url).await.unwrap();
    let store = SqliteStore::new(pool.clone());
    store.migrate().await.unwrap();

    let webauthn = WebauthnBuilder::new("localhost", &Url::parse("http://localhost:8080").unwrap())
        .unwrap()
        .rp_name("test")
        .build()
        .unwrap();

    let state = AppState {
        db: pool,
        webauthn: Arc::new(webauthn),
        export_token: Arc::from("test-token"),
    };
    build_router(state, store, false)
}

#[tokio::test]
async fn healthz_ok() {
    let app = test_app().await;
    let res = app
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn export_requires_token() {
    let app = test_app().await;
    let res = app
        .oneshot(Request::builder().uri("/api/export").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn export_with_token_returns_empty_array() {
    let app = test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/export")
                .header(header::AUTHORIZATION, "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"[]");
}

#[tokio::test]
async fn signup_requires_authenticated_session() {
    let app = test_app().await;
    let payload = r#"{"full_name":"A","company":"B","street":"C","postal_code":"1","city":"D","country":"E","gdpr_consent":true}"#;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

async fn get(app: Router, uri: &str) -> axum::response::Response {
    app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn serves_embedded_index() {
    let res = get(test_app().await, "/").await;
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("text/html"), "content-type was {ct}");
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("Firefox"));
}

#[tokio::test]
async fn serves_embedded_css() {
    let res = get(test_app().await, "/styles.css").await;
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(ct.contains("text/css"), "content-type was {ct}");
}

#[tokio::test]
async fn unknown_asset_is_404() {
    let res = get(test_app().await, "/does-not-exist.js").await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn register_start_returns_challenge() {
    let app = test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/register/start")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"email":"new@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let txt = String::from_utf8_lossy(&body);
    assert!(
        txt.contains("publicKey") && txt.contains("challenge"),
        "body was {txt}"
    );
}

#[tokio::test]
async fn login_start_unknown_email_hints_register() {
    let app = test_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login/start")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"email":"nobody@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("register"));
}

#[tokio::test]
async fn get_signup_requires_session() {
    let res = get(test_app().await, "/api/signup").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn export_includes_registered_without_signup() {
    let path = std::env::temp_dir().join(format!("fu-test-{}.db", Uuid::new_v4()));
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let pool = crate::db::init_pool(&url).await.unwrap();

    // One user who only registered a key.
    let only_registered = Uuid::new_v4();
    sqlx::query("INSERT INTO users (user_id, email, created_at) VALUES (?, ?, ?)")
        .bind(only_registered)
        .bind("registered@example.com")
        .bind("2026-01-01T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

    // One user who also completed the signup form.
    let signed = Uuid::new_v4();
    sqlx::query("INSERT INTO users (user_id, email, created_at) VALUES (?, ?, ?)")
        .bind(signed)
        .bind("signed@example.com")
        .bind("2026-01-02T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO signups (user_id, email, full_name, company, street, postal_code, city, country, gdpr_consent, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(signed)
    .bind("signed@example.com")
    .bind("Ada Lovelace")
    .bind("ACME")
    .bind("Main St 1")
    .bind("12345")
    .bind("Berlin")
    .bind("DE")
    .bind(true)
    .bind("2026-01-03T00:00:00Z")
    .execute(&pool)
    .await
    .unwrap();

    let rows = crate::db::all_registrations(&pool).await.unwrap();
    assert_eq!(rows.len(), 2, "export should include every registered key");

    let reg = rows.iter().find(|r| r.email == "registered@example.com").unwrap();
    assert!(!reg.signed_up);
    assert!(reg.full_name.is_none());

    let sig = rows.iter().find(|r| r.email == "signed@example.com").unwrap();
    assert!(sig.signed_up);
    assert_eq!(sig.full_name.as_deref(), Some("Ada Lovelace"));
    assert_eq!(sig.gdpr_consent, Some(true));
}
