use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{header, HeaderMap},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::{load_current_user, CurrentSession};
use crate::error::AppError;
use crate::middleware::RequestId;
use crate::models::{SystemAccess, SystemStatus};
use crate::services::audit::{write_audit, AuditPayload};
use crate::services::permissions;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/portal/home", get(home))
        .route("/api/portal/systems/{system_code}/enter", post(enter))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HomeQuery {
    tenant_id: Option<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HomeResponse {
    current_tenant: Option<TenantSummary>,
    available_tenants: Vec<TenantSummary>,
    user: UserSummary,
    groups: Vec<Group>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_tenant: Option<bool>,
    allow_permission_request: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TenantSummary {
    id: Uuid,
    code: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSummary {
    id: Uuid,
    display_name: String,
    avatar_url: Option<String>,
    can_enter_admin: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Group {
    key: String,
    title: String,
    systems: Vec<SystemSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemSummary {
    system_code: String,
    name: String,
    description: Option<String>,
    icon_url: Option<String>,
    status: SystemStatus,
    visible: bool,
    accessible: bool,
    identity_label: String,
    tenant_label: String,
    permission_summary: Vec<String>,
    enterable: bool,
}

async fn home(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(query): Query<HomeQuery>,
) -> Result<Json<HomeResponse>, AppError> {
    let current_user = load_current_user(&state, session.user.id).await?;
    let available_tenants: Vec<TenantSummary> = current_user
        .tenants
        .iter()
        .map(|t| TenantSummary {
            id: t.id,
            code: t.code.clone(),
            name: t.name.clone(),
        })
        .collect();

    if available_tenants.is_empty() {
        return Ok(Json(HomeResponse {
            current_tenant: None,
            available_tenants: vec![],
            user: UserSummary {
                id: current_user.id,
                display_name: current_user.display_name,
                avatar_url: current_user.avatar_url,
                can_enter_admin: current_user.can_enter_admin,
            },
            groups: vec![],
            no_tenant: Some(true),
            allow_permission_request: state.config.allow_permission_request,
        }));
    }

    let requested = query
        .tenant_id
        .or(current_user.default_tenant_id)
        .and_then(|id| available_tenants.iter().find(|t| t.id == id).cloned());
    let current_tenant = requested.or_else(|| available_tenants.first().cloned()).unwrap();

    let contexts = permissions::list_user_system_access(&state.db, session.user.id, current_tenant.id).await?;
    let frequent_codes = frequent_systems(&session.user.preferences);

    let map_system = |c: &SystemAccess| SystemSummary {
        system_code: c.system_code.clone(),
        name: c.name.clone(),
        description: c.description.clone(),
        icon_url: c.icon_url.clone(),
        status: c.status.clone(),
        visible: c.visible,
        accessible: c.accessible,
        identity_label: c.identity_label.clone(),
        tenant_label: current_tenant.name.clone(),
        permission_summary: c.permissions.iter().take(3).cloned().collect(),
        enterable: c.accessible,
    };

    let frequent: Vec<SystemSummary> = contexts
        .iter()
        .filter(|c| {
            frequent_codes.contains(&c.system_code)
                && matches!(c.status, SystemStatus::Active)
                && c.accessible
        })
        .map(map_system)
        .collect();

    let all: Vec<SystemSummary> = contexts
        .iter()
        .filter(|c| matches!(c.status, SystemStatus::Active | SystemStatus::Onboarding))
        .map(map_system)
        .collect();

    let maintenance: Vec<SystemSummary> = contexts
        .iter()
        .filter(|c| matches!(c.status, SystemStatus::Maintenance))
        .map(map_system)
        .collect();

    Ok(Json(HomeResponse {
        current_tenant: Some(current_tenant.clone()),
        available_tenants,
        user: UserSummary {
            id: current_user.id,
            display_name: current_user.display_name,
            avatar_url: current_user.avatar_url,
            can_enter_admin: current_user.can_enter_admin,
        },
        groups: vec![
            Group {
                key: "frequent".to_string(),
                title: "常用系统".to_string(),
                systems: frequent,
            },
            Group {
                key: "all".to_string(),
                title: "全部系统".to_string(),
                systems: all,
            },
            Group {
                key: "maintenance".to_string(),
                title: "维护中系统".to_string(),
                systems: maintenance,
            },
        ],
        no_tenant: None,
        allow_permission_request: state.config.allow_permission_request,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnterRequest {
    tenant_id: Option<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnterResponse {
    callback_url: String,
    code: String,
    expires_at: chrono::DateTime<Utc>,
}

async fn enter(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    session: CurrentSession,
    axum::extract::Path(system_code): axum::extract::Path<String>,
    Json(req): Json<EnterRequest>,
) -> Result<Json<EnterResponse>, AppError> {
    let system_row = sqlx::query(
        r#"SELECT id, code, name, callback_url, entry_url, status as "status: SystemStatus"
           FROM portal_systems WHERE code = $1"#,
    )
    .bind(&system_code)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("system query failed: {}", e)))?;

    let system_row = match system_row {
        Some(r) => r,
        None => return Err(AppError::SystemDisabled),
    };

    let system_id: Uuid = system_row.get("id");

    let memberships: Vec<(Uuid,)> = sqlx::query_as(
        r#"SELECT tenant_id FROM portal_tenant_members
           WHERE user_id = $1 AND member_status = 'active'"#,
    )
    .bind(session.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("membership query failed: {}", e)))?;

    let tenant_id = if let Some(tid) = req.tenant_id {
        tid
    } else if let Some(tid) = session.user.default_tenant_id {
        tid
    } else if let Some(first) = memberships.first() {
        first.0
    } else {
        return Err(AppError::TenantDisabled);
    };

    if !memberships.iter().any(|m| m.0 == tenant_id) {
        return Err(AppError::TenantDisabled);
    }

    let access = permissions::get_user_system_access(&state.db, session.user.id, system_id, tenant_id)
        .await?;

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let accessible = access.as_ref().map(|a| a.accessible).unwrap_or(false);
    if !accessible {
        write_audit(
            &state.db,
            AuditPayload {
                request_id: Some(request_id.0.clone()),
                actor_user_id: Some(session.user.id),
                action: "subsystem.ticket.issue".to_string(),
                target_type: "system".to_string(),
                target_id: Some(system_id.to_string()),
                system_id: Some(system_id),
                tenant_id: Some(tenant_id),
                result: "failure",
                before_data: None,
                after_data: None,
                failure_reason: Some("not_accessible".to_string()),
                ip_address: Some(ip.clone()),
                user_agent: user_agent.clone(),
            },
        )
        .await
        .ok();
        return Err(AppError::PermissionDenied);
    }

    let access = access.unwrap();

    let roles: Vec<String> = sqlx::query_scalar(
        r#"SELECT r.code FROM portal_user_roles ur
           JOIN portal_roles r ON r.id = ur.role_id
           WHERE ur.user_id = $1"#,
    )
    .bind(session.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("roles query failed: {}", e)))?;

    let issued_at = Utc::now();
    let expires_at = issued_at + Duration::seconds(state.config.jwt.token_ttl_seconds);

    let context_snapshot = serde_json::json!({
        "userId": session.user.id,
        "username": session.user.username,
        "displayName": session.user.display_name,
        "email": session.user.email,
        "avatarUrl": session.user.avatar_url,
        "tenantId": tenant_id,
        "systemCode": system_code,
        "portalRoles": roles,
        "systemRoles": access.system_roles,
        "permissions": access.permissions,
        "adminScopes": access.scopes,
        "issuedAt": issued_at.timestamp(),
        "expiresAt": expires_at.timestamp(),
    });

    let ticket_id = Uuid::new_v4();
    let secret = generate_random_token(48);
    let code_hash = bcrypt::hash(&secret, 10)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("ticket hash failed: {}", e)))?;

    sqlx::query(
        r#"INSERT INTO portal_subsystem_tickets
           (id, code_hash, user_id, tenant_id, system_id, context_snapshot, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(ticket_id)
    .bind(code_hash)
    .bind(session.user.id)
    .bind(tenant_id)
    .bind(system_id)
    .bind(context_snapshot)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("ticket insert failed: {}", e)))?;

    write_audit(
        &state.db,
        AuditPayload {
            request_id: Some(request_id.0),
            actor_user_id: Some(session.user.id),
            action: "subsystem.ticket.issue".to_string(),
            target_type: "system".to_string(),
            target_id: Some(system_id.to_string()),
            system_id: Some(system_id),
            tenant_id: Some(tenant_id),
            result: "success",
            before_data: None,
            after_data: Some(serde_json::json!({
                "systemCode": system_code,
                "tenantId": tenant_id,
            })),
            failure_reason: None,
            ip_address: Some(ip),
            user_agent,
        },
    )
    .await
    .ok();

    let callback_url = system_row
        .get::<Option<String>, _>("callback_url")
        .unwrap_or_else(|| system_row.get::<String, _>("entry_url"));

    Ok(Json(EnterResponse {
        callback_url,
        code: format!("{}:{}", ticket_id, secret),
        expires_at,
    }))
}

fn frequent_systems(preferences: &Value) -> Vec<String> {
    preferences
        .get("frequentSystems")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn generate_random_token(length: usize) -> String {
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}
