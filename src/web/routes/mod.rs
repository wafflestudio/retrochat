use axum::{routing::get, Router};
use tower_http::services::ServeDir;

use crate::web::handlers;

pub fn create_routes() -> Router {
    // API routes
    let api_routes = Router::new()
        .route("/sessions", get(handlers::list_sessions))
        .route("/sessions/:id", get(handlers::get_session_detail))
        .route("/search", get(handlers::search_messages))
        .route("/timeline", get(handlers::query_timeline))
        .route("/health", get(handlers::health_check));

    // Static files - serve from embedded directory at compile time
    let static_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("web")
        .join("static");

    Router::new()
        .nest("/api", api_routes)
        .nest_service("/", ServeDir::new(static_dir))
}
