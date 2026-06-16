use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use uuid::Uuid;

use crate::auth::load_current_user;
use crate::error::AppError;
use crate::services::audit::{write_audit, AuditPayload};
use crate::services::auth::{create_session, hash_password};
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupStatusResponse {
    initialized: bool,
    needs_setup: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapRequest {
    username: String,
    display_name: String,
    email: Option<String>,
    password: String,
    tenant_name: Option<String>,
}

#[derive(Serialize)]
struct BootstrapResponse {
    user: crate::models::CurrentUser,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/setup/status", get(status))
        .route("/api/setup/bootstrap-super-admin", post(bootstrap_super_admin))
}

async fn status(State(state): State<AppState>) -> Result<Json<SetupStatusResponse>, AppError> {
    let initialized = has_super_admin(&state).await?;
    Ok(Json(SetupStatusResponse {
        initialized,
        needs_setup: !initialized,
    }))
}

async fn bootstrap_super_admin(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<BootstrapRequest>,
) -> Result<Response, AppError> {
    if has_super_admin(&state).await? {
        return Err(AppError::Conflict("PORTAL_ALREADY_INITIALIZED".to_string()));
    }

    let username = req.username.trim();
    let display_name = req.display_name.trim();
    let password = req.password.trim();
    if username.is_empty() || display_name.is_empty() || password.len() < 8 {
        return Err(AppError::ValidationFailed(
            "USERNAME_DISPLAY_NAME_AND_PASSWORD_REQUIRED".to_string(),
        ));
    }

    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM portal_users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("username check failed: {}", e)))?;
    if existing.is_some() {
        return Err(AppError::Conflict("USERNAME_EXISTS".to_string()));
    }

    ensure_builtin_roles(&state).await?;
    let tenant_id = ensure_default_tenant(&state, req.tenant_name.as_deref()).await?;
    let role_id: Uuid = sqlx::query_scalar("SELECT id FROM portal_roles WHERE code = 'super-admin'")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("load super-admin role failed: {}", e)))?;

    let user_id = Uuid::new_v4();
    let password_hash = hash_password(password)?;
    sqlx::query(
        r#"INSERT INTO portal_users
           (id, username, password_hash, display_name, email, status, default_tenant_id, preferences)
           VALUES ($1, $2, $3, $4, $5, 'active'::"UserStatus", $6, '{}')"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(password_hash)
    .bind(display_name)
    .bind(req.email.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
    .bind(tenant_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create super admin failed: {}", e)))?;

    sqlx::query(
        "INSERT INTO portal_tenant_members (id, tenant_id, user_id, member_status) VALUES ($1, $2, $3, 'active') ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(tenant_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create tenant membership failed: {}", e)))?;

    sqlx::query(
        "INSERT INTO portal_user_roles (id, user_id, role_id, tenant_id) VALUES ($1, $2, $3, NULL) ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(role_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("assign super admin role failed: {}", e)))?;

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    write_audit(
        &state.db,
        AuditPayload {
            request_id: None,
            actor_user_id: Some(user_id),
            action: "setup.bootstrap_super_admin".to_string(),
            target_type: "user".to_string(),
            target_id: Some(user_id.to_string()),
            system_id: None,
            tenant_id: Some(tenant_id),
            result: "success",
            before_data: None,
            after_data: Some(json!({
                "username": username,
                "displayName": display_name,
                "tenantId": tenant_id,
                "role": "super-admin"
            })),
            failure_reason: None,
            ip_address: Some(ip.clone()),
            user_agent: user_agent.clone(),
        },
    )
    .await
    .ok();

    let (session_id, secret) = create_session(
        &state.db,
        user_id,
        false,
        state.config.session.ttl_seconds,
        state.config.session.remember_me_ttl_seconds,
        Some(ip),
        user_agent,
    )
    .await?;
    let cookie_value = format!("{}:{}; Path=/; HttpOnly; SameSite=Lax", session_id, secret);
    let cookie = format!("{}={}", state.config.session.cookie_name, cookie_value);
    let user = load_current_user(&state, user_id).await?;

    Ok((
        StatusCode::OK,
        AppendHeaders([(header::SET_COOKIE, cookie)]),
        Json(BootstrapResponse { user }),
    )
        .into_response())
}

async fn has_super_admin(state: &AppState) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar(
        r#"SELECT EXISTS (
            SELECT 1
            FROM portal_users u
            JOIN portal_user_roles ur ON ur.user_id = u.id
            JOIN portal_roles r ON r.id = ur.role_id
            WHERE u.status = 'active'::"UserStatus" AND r.code = 'super-admin'
        )"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("setup status query failed: {}", e)))?;
    Ok(exists)
}

async fn ensure_builtin_roles(state: &AppState) -> Result<(), AppError> {
    let roles = [
        ("super-admin", "超级管理员", "super_admin"),
        ("user", "普通用户", "normal"),
        ("subsystem-admin", "子系统管理员", "subsystem_admin"),
        ("audit-viewer", "审计查看员", "custom"),
        ("system-integrator", "系统集成员", "custom"),
    ];
    for (code, name, role_type) in roles {
        sqlx::query(
            r#"INSERT INTO portal_roles (id, code, name, role_type, is_builtin)
               VALUES ($1, $2, $3, $4::"RoleType", true)
               ON CONFLICT (code) DO NOTHING"#,
        )
        .bind(Uuid::new_v4())
        .bind(code)
        .bind(name)
        .bind(role_type)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("ensure builtin role failed: {}", e)))?;
    }
    Ok(())
}

async fn ensure_default_tenant(state: &AppState, tenant_name: Option<&str>) -> Result<Uuid, AppError> {
    let tenant_id = Uuid::new_v4();
    let name = tenant_name
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("默认租户");
    sqlx::query(
        r#"INSERT INTO portal_tenants (id, code, name, status, description)
           VALUES ($1, 'default', $2, 'active'::"TenantStatus", '系统默认租户')
           ON CONFLICT (code) DO UPDATE
           SET name = EXCLUDED.name,
               status = EXCLUDED.status,
               updated_at = NOW()"#,
    )
    .bind(tenant_id)
    .bind(name)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("ensure default tenant failed: {}", e)))?;

    sqlx::query_scalar("SELECT id FROM portal_tenants WHERE code = 'default'")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("load default tenant failed: {}", e)))
}
