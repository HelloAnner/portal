pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod models;
pub mod permissions;
pub mod response;
pub mod services;
pub mod state;

use std::net::SocketAddr;

use axum::{
    body::Body,
    http::{Response, StatusCode},
    middleware::from_fn,
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use axum::http::Method;
use tracing::info;

use crate::config::AppConfig;
use crate::db::{connect_database, run_migrations};
use crate::middleware::{audit_middleware, request_id_middleware};
use crate::response::EnvelopeLayer;
use crate::state::AppState;

pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();
    info!("starting portal-api on port {}", config.app_port);

    let db = connect_database(&config).await?;
    run_migrations(&db).await?;

    let state = AppState::new(config.clone(), db);

    let app = router(state.clone()).with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.app_port));
    let listener = TcpListener::bind(&addr).await?;
    info!("listening on {}", addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

pub fn router(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/ready", get(ready))
        .merge(api::auth_router())
        .merge(api::portal_router())
        .merge(api::profile_router())
        .merge(api::me_router())
        .merge(api::admin_router())
        .fallback(static_or_spa)
        .layer(EnvelopeLayer)
        .layer(from_fn(audit_middleware))
        .layer(from_fn(request_id_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::mirror_request())
                .allow_methods(AllowMethods::list([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ]))
                .allow_headers(AllowHeaders::list([
                    "content-type".parse().unwrap(),
                    "authorization".parse().unwrap(),
                    "x-request-id".parse().unwrap(),
                ]))
                .allow_credentials(true),
        )
        .layer(TraceLayer::new_for_http())
}

async fn health() -> &'static str {
    "{\"ok\":true}"
}

async fn ready() -> &'static str {
    "{\"ok\":true}"
}

async fn static_or_spa(req: axum::http::Request<Body>) -> Response<Body> {
    let path = req.uri().path();
    if let Some(asset) = web_embed::get_asset(path) {
        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", asset.content_type)
            .body(Body::from(asset.bytes))
            .unwrap();
    }

    // SPA fallback: return index.html for non-API routes
    if let Some(asset) = web_embed::get_asset("/index.html") {
        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", asset.content_type)
            .body(Body::from(asset.bytes))
            .unwrap();
    }

    let fallback = web_embed::fallback_html();
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", fallback.content_type)
        .body(Body::from(fallback.bytes))
        .unwrap()
}
