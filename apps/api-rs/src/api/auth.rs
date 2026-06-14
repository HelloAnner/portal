use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use bcrypt::verify;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::net::SocketAddr;
use uuid::Uuid;

use crate::auth::{load_current_user, CurrentSession};
use crate::error::AppError;
use crate::models::{CurrentUser, UserStatus};
use crate::services::auth::{create_session, revoke_session};
use crate::services::audit::{write_audit, AuditPayload};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user: CurrentUser,
}

#[derive(Deserialize)]
pub struct ExchangeTicketRequest {
    pub code: String,
    pub system_code: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/api/auth/exchange-ticket", post(exchange_ticket))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let remember_me = req.remember_me.unwrap_or(false);

    let row = sqlx::query(
        r#"SELECT id, username, password_hash, display_name, email, avatar_url, status as "status"
           FROM portal_users WHERE username = $1"#,
    )
    .bind(&req.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("login query failed: {}", e)))?;

    let row = row.ok_or(AppError::AuthRequired)?;
    let user_id: Uuid = row.get("id");
    let password_hash: Option<String> = row.get("password_hash");
    let hash = password_hash.ok_or(AppError::AuthRequired)?;

    let valid = verify(&req.password, &hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("password verify error")))?;
    if !valid {
        return Err(AppError::AuthRequired);
    }

    if !matches!(row.get::<UserStatus, _>("status"), UserStatus::Active) {
        return Err(AppError::AuthRequired);
    }

    sqlx::query("UPDATE portal_users SET last_login_at = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("update last login failed: {}", e)))?;

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let (session_id, secret) = create_session(
        &state.db,
        user_id,
        remember_me,
        state.config.session.ttl_seconds,
        state.config.session.remember_me_ttl_seconds,
        Some(ip.clone()),
        user_agent.clone(),
    )
    .await?;

    let cookie_value = format!("{}:{}; Path=/; HttpOnly; SameSite=Lax", session_id, secret);
    let cookie = format!("{}={}", state.config.session.cookie_name, cookie_value);

    write_audit(
        &state.db,
        AuditPayload {
            request_id: None,
            actor_user_id: Some(user_id),
            action: "login".to_string(),
            target_type: "user".to_string(),
            target_id: Some(user_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: None,
            failure_reason: None,
            ip_address: Some(ip),
            user_agent,
        },
    )
    .await
    .ok();

    let current_user = load_current_user(&state, user_id).await?;
    let body = Json(LoginResponse { user: current_user });

    Ok((
        StatusCode::OK,
        AppendHeaders([(header::SET_COOKIE, cookie)]),
        body,
    )
        .into_response())
}

async fn logout(
    State(state): State<AppState>,
    session: CurrentSession,
) -> Result<Response, AppError> {
    revoke_session(&state.db, session.session_id).await?;

    let clear_cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        state.config.session.cookie_name
    );

    Ok((
        StatusCode::OK,
        AppendHeaders([(header::SET_COOKIE, clear_cookie)]),
        Json(serde_json::json!({"ok": true})),
    )
        .into_response())
}

async fn me(
    State(state): State<AppState>,
    session: CurrentSession,
) -> Result<Json<CurrentUser>, AppError> {
    let user = load_current_user(&state, session.user.id).await?;
    Ok(Json(user))
}

async fn exchange_ticket(
    State(state): State<AppState>,
    Json(req): Json<ExchangeTicketRequest>,
) -> Result<Json<Value>, AppError> {
    // code format: uuid:secret
    let (ticket_id_str, secret) = req.code.split_once(':').ok_or(AppError::InvalidSubsystemTicket)?;
    let ticket_id = ticket_id_str.parse::<Uuid>().map_err(|_| AppError::InvalidSubsystemTicket)?;

    let row = sqlx::query(
        r#"SELECT t.id, t.code_hash, t.user_id, t.tenant_id, t.system_id, t.context_snapshot,
                  t.expires_at, t.consumed_at, s.code as system_code
           FROM portal_subsystem_tickets t
           JOIN portal_systems s ON s.id = t.system_id
           WHERE t.id = $1"#,
    )
    .bind(ticket_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("ticket query failed: {}", e)))?;

    let row = row.ok_or(AppError::InvalidSubsystemTicket)?;
    let hash: String = row.get("code_hash");
    let expires_at: chrono::DateTime<Utc> = row.get("expires_at");
    let consumed_at: Option<chrono::DateTime<Utc>> = row.get("consumed_at");
    let system_code: String = row.get("system_code");

    if consumed_at.is_some() || expires_at < Utc::now() {
        return Err(AppError::InvalidSubsystemTicket);
    }

    if system_code != req.system_code {
        return Err(AppError::InvalidSubsystemTicket);
    }

    let valid = verify(secret, &hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("ticket verify error")))?;
    if !valid {
        return Err(AppError::InvalidSubsystemTicket);
    }

    sqlx::query("UPDATE portal_subsystem_tickets SET consumed_at = NOW() WHERE id = $1")
        .bind(ticket_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("consume ticket failed: {}", e)))?;

    let snapshot: Value = row.get("context_snapshot");
    Ok(Json(snapshot))
}
