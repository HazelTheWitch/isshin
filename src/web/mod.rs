pub mod auth;

use std::sync::Arc;

use axum::{routing::get, Json, Router};

use axum::{error_handling::HandleErrorLayer, http::StatusCode, BoxError};
use axum_login::{
    login_required,
    tower_sessions::{cookie::time::Duration, Expiry, MemoryStore, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use sqlx::SqlitePool;
use tower::ServiceBuilder;

use crate::state::AppState;
use crate::user::Backend;

pub fn protected() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/test",
            get(|| async { Json(serde_json::json!({ "status": "UP" })) }),
        )
        .merge(auth::protected())
}

pub async fn app(db: SqlitePool) -> anyhow::Result<Router> {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(5)));

    let backend = Backend::new(db.clone());
    let auth_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|_: BoxError| async {
            StatusCode::BAD_REQUEST
        }))
        .layer(AuthManagerLayerBuilder::new(backend, session_layer).build());

    let app = protected()
        .route_layer(login_required!(Backend, login_url = "/login"))
        .merge(auth::router())
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({ "status": "UP" })) }),
        )
        .layer(auth_service)
        .with_state(Arc::new(AppState { db }));

    Ok(app)
}
