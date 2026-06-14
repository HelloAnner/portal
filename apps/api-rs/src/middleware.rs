use axum::{
    extract::{ConnectInfo, Request},
    http::header,
    middleware::Next,
    response::Response,
};
use rand::Rng;
use ipnetwork::IpNetwork;
use std::net::SocketAddr;
use std::time::Instant;
use tracing::info;

use crate::state::AppState;

pub fn generate_request_id() -> String {
    let mut rng = rand::thread_rng();
    let id: u64 = rng.gen();
    format!("req_{:016x}", id)
}

#[derive(Clone, Debug)]
pub struct RequestId(pub String);

pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_request_id);
    req.extensions_mut().insert(request_id.clone());
    req.extensions_mut().insert(RequestId(request_id.clone()));
    let mut response = next.run(req).await;
    response.headers_mut().insert("x-request-id", request_id.parse().unwrap());
    response
}

pub async fn audit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let request_id = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(generate_request_id);
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let state = req.extensions().get::<AppState>().cloned();
    let actor_user_id = req.extensions().get::<String>().cloned(); // set by auth extractor

    let response = next.run(req).await;
    let duration = start.elapsed();
    let status = response.status();

    let result = if status.is_client_error() || status.is_server_error() {
        "failure"
    } else {
        "success"
    };

    if let Some(state) = state {
        if should_audit(&method, &path) {
            let ip: Option<IpNetwork> = addr.ip().to_string().parse().ok();
            let action = format!("{} {}", method, path);
            let _ = sqlx::query(
                r#"INSERT INTO portal_audit_events
                   (id, request_id, actor_user_id, action, target_type, target_id, result, ip_address, user_agent)
                   VALUES ($1, $2, $3::uuid, $4, $5, $6, $7::AuditResult, $8, $9)"#,
            )
            .bind(uuid::Uuid::new_v4())
            .bind(&request_id)
            .bind(actor_user_id)
            .bind(&action)
            .bind("http")
            .bind(&path)
            .bind(result)
            .bind(ip.map(|n| n.to_string()))
            .bind(user_agent)
            .execute(&state.db)
            .await;
        }
    }

    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "request"
    );

    response
}

fn should_audit(method: &str, path: &str) -> bool {
    if path.starts_with("/api/health") || path.starts_with("/api/ready") {
        return false;
    }
    matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
}
