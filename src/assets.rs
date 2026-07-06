//! Static frontend, compiled into the binary via `rust-embed`.

use axum::body::Body;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/"]
struct Frontend;

/// `GET /` — serve the landing page.
pub async fn index() -> Response {
    serve("index.html")
}

/// Fallback handler — serve any other embedded asset by path.
pub async fn static_handler(uri: Uri) -> Response {
    serve(uri.path().trim_start_matches('/'))
}

fn serve(path: &str) -> Response {
    match Frontend::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let cache = if path.starts_with("assets/") {
                "public, max-age=86400"
            } else {
                "no-cache"
            };
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, cache)
                .body(Body::from(content.data.into_owned()))
                .expect("valid response")
        }
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}
