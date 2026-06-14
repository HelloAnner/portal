use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header, HeaderMap},
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::CurrentSession;
use crate::error::AppError;
use crate::middleware::RequestId;
use crate::models::{AdminScope, SystemStatus, TenantStatus};
use crate::services::audit::{write_audit, AuditPayload};
use crate::services::permissions;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/me/permissions", get(permissions))
        .route("/api/me/permissions/{system_code}", get(permission_detail))
        .route("/api/access-denied/context", get(access_denied_context))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TenantSummary {
    id: Uuid,
    code: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionsResponse {
    portal_roles: Vec<String>,
    tenants: Vec<TenantSummary>,
    systems: Vec<PermissionSystemItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionSystemItem {
    system_id: Uuid,
    system_code: String,
    name: String,
    description: Option<String>,
    icon_url: Option<String>,
    status: SystemStatus,
    category: Option<String>,
    tenant_id: Uuid,
    tenant_name: String,
    identity: String,
    visible: bool,
    accessible: bool,
    source_summary: Vec<String>,
    scope_summary: Vec<String>,
}

async fn permissions(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    session: CurrentSession,
) -> Result<Json<PermissionsResponse>, AppError> {
    let roles: Vec<String> = sqlx::query_scalar(
        r#"SELECT DISTINCT r.code
           FROM portal_user_roles ur
           JOIN portal_roles r ON r.id = ur.role_id
           WHERE ur.user_id = $1"#,
    )
    .bind(session.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("roles query failed: {}", e)))?;

    let memberships: Vec<(Uuid, String, String)> = sqlx::query_as(
        r#"SELECT t.id, t.code, t.name
           FROM portal_tenant_members tm
           JOIN portal_tenants t ON t.id = tm.tenant_id
           WHERE tm.user_id = $1 AND tm.member_status = 'active' AND t.status = 'active'
           ORDER BY tm.joined_at ASC"#,
    )
    .bind(session.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("memberships query failed: {}", e)))?;

    let systems: Vec<(Uuid, String, String, Option<String>, Option<String>, Option<String>, SystemStatus)> =
        sqlx::query_as(
            r#"SELECT id, code, name, description, icon_url, category, status as "status: SystemStatus"
               FROM portal_systems
               WHERE status IN ('active', 'maintenance', 'onboarding')"#,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("systems query failed: {}", e)))?;

    let mut system_items = Vec::new();
    for (tenant_id, _tenant_code, tenant_name) in &memberships {
        for (system_id, system_code, name, description, icon_url, category, status) in &systems {
            let details = permissions::get_permission_details(
                &state.db,
                session.user.id,
                *system_id,
                *tenant_id,
            )
            .await?;
            if !details.context.visible {
                continue;
            }
            system_items.push(PermissionSystemItem {
                system_id: *system_id,
                system_code: system_code.clone(),
                name: name.clone(),
                description: description.clone(),
                icon_url: icon_url.clone(),
                status: status.clone(),
                category: category.clone(),
                tenant_id: *tenant_id,
                tenant_name: tenant_name.clone(),
                identity: details.context.identity_label.clone(),
                visible: details.context.visible,
                accessible: details.context.accessible,
                source_summary: details.source_summary,
                scope_summary: details.scope_summary,
            });
        }
    }

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    write_audit(
        &state.db,
        AuditPayload {
            request_id: Some(request_id.0),
            actor_user_id: Some(session.user.id),
            action: "permission.view".to_string(),
            target_type: "user".to_string(),
            target_id: Some(session.user.id.to_string()),
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

    Ok(Json(PermissionsResponse {
        portal_roles: roles,
        tenants: memberships
            .into_iter()
            .map(|(id, code, name)| TenantSummary { id, code, name })
            .collect(),
        systems: system_items,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionDetailResponse {
    system_code: String,
    name: String,
    description: Option<String>,
    icon_url: Option<String>,
    status: SystemStatus,
    category: Option<String>,
    contexts: Vec<ContextDetail>,
    aggregate: AggregateDetail,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextDetail {
    tenant_id: Uuid,
    tenant_name: String,
    identity: String,
    visible: bool,
    accessible: bool,
    permissions: Vec<String>,
    admin_scopes: Vec<AdminScope>,
    sources: Vec<PermissionSource>,
    source_summary: Vec<String>,
    scope_summary: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionSource {
    #[serde(rename = "type")]
    source_type: String,
    label: String,
    visible: bool,
    accessible: bool,
    system_roles: Vec<String>,
    permissions: Vec<String>,
    admin_scopes: Vec<AdminScope>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AggregateDetail {
    permissions: Vec<String>,
    admin_scopes: Vec<AdminScope>,
    scope_summary: Vec<String>,
    context_preview: String,
}

async fn permission_detail(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    session: CurrentSession,
    Path(system_code): Path<String>,
) -> Result<Json<PermissionDetailResponse>, AppError> {
    let system_row = sqlx::query(
        r#"SELECT id, code, name, description, icon_url, category, status as "status: SystemStatus"
           FROM portal_systems WHERE code = $1"#,
    )
    .bind(&system_code)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("system query failed: {}", e)))?;

    let system_row = match system_row {
        Some(r) => r,
        None => return Err(AppError::NotFound),
    };

    let system_id: Uuid = system_row.get("id");

    let memberships: Vec<(Uuid, String, String)> = sqlx::query_as(
        r#"SELECT t.id, t.code, t.name
           FROM portal_tenant_members tm
           JOIN portal_tenants t ON t.id = tm.tenant_id
           WHERE tm.user_id = $1 AND tm.member_status = 'active' AND t.status = 'active'
           ORDER BY tm.joined_at ASC"#,
    )
    .bind(session.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("memberships query failed: {}", e)))?;

    let mut contexts = Vec::new();
    let mut all_permissions: HashSet<String> = HashSet::new();
    let mut all_scopes: HashMap<String, AdminScope> = HashMap::new();
    let mut all_scope_summary: HashSet<String> = HashSet::new();
    let mut preview_parts = Vec::new();

    for (tenant_id, _tenant_code, tenant_name) in &memberships {
        let details = permissions::get_permission_details(
            &state.db,
            session.user.id,
            system_id,
            *tenant_id,
        )
        .await?;
        if !details.context.visible {
            continue;
        }

        for p in &details.context.permissions {
            all_permissions.insert(p.clone());
        }
        for s in &details.context.admin_scopes {
            all_scopes.insert(format!("{}:{}", s.scope_type, s.scope_code), s.clone());
        }
        for s in &details.scope_summary {
            all_scope_summary.insert(s.clone());
        }

        let status_label = if details.context.accessible {
            "（可进入）"
        } else {
            "（仅可见）"
        };
        preview_parts.push(format!(
            "{}：{}{}",
            tenant_name, details.context.identity_label, status_label
        ));

        contexts.push(ContextDetail {
            tenant_id: *tenant_id,
            tenant_name: tenant_name.clone(),
            identity: details.context.identity_label,
            visible: details.context.visible,
            accessible: details.context.accessible,
            permissions: details.context.permissions,
            admin_scopes: details.context.admin_scopes,
            sources: details
                .sources
                .into_iter()
                .map(|s| PermissionSource {
                    source_type: s.source_type,
                    label: s.label,
                    visible: s.visible,
                    accessible: s.accessible,
                    system_roles: s.system_roles,
                    permissions: s.permissions,
                    admin_scopes: s.admin_scopes,
                })
                .collect(),
            source_summary: details.source_summary,
            scope_summary: details.scope_summary,
        });
    }

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    write_audit(
        &state.db,
        AuditPayload {
            request_id: Some(request_id.0),
            actor_user_id: Some(session.user.id),
            action: "permission.view".to_string(),
            target_type: "system".to_string(),
            target_id: Some(system_id.to_string()),
            system_id: Some(system_id),
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: Some(serde_json::json!({"systemCode": system_code})),
            failure_reason: None,
            ip_address: Some(ip),
            user_agent,
        },
    )
    .await
    .ok();

    Ok(Json(PermissionDetailResponse {
        system_code: system_row.get("code"),
        name: system_row.get("name"),
        description: system_row.get("description"),
        icon_url: system_row.get("icon_url"),
        status: system_row.get("status"),
        category: system_row.get("category"),
        contexts,
        aggregate: AggregateDetail {
            permissions: all_permissions.into_iter().collect(),
            admin_scopes: all_scopes.into_values().collect(),
            scope_summary: all_scope_summary.into_iter().collect(),
            context_preview: preview_parts.join("；"),
        },
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccessDeniedQuery {
    reason: Option<String>,
    system_code: Option<String>,
    tenant_id: Option<Uuid>,
    return_to: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccessDeniedContextResponse {
    reason: String,
    title: String,
    description: String,
    recovery_action: String,
    return_to: String,
    system: Option<SystemSummary>,
    tenant: Option<AccessDeniedTenantSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccessDeniedTenantSummary {
    id: Uuid,
    code: String,
    name: String,
    status: TenantStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemSummary {
    id: Uuid,
    code: String,
    name: String,
    status: SystemStatus,
}

async fn access_denied_context(
    State(state): State<AppState>,
    Query(query): Query<AccessDeniedQuery>,
) -> Result<Json<AccessDeniedContextResponse>, AppError> {
    let reason = query.reason.unwrap_or_else(|| "no_permission".to_string());
    let return_to = query.return_to.unwrap_or_else(|| "/".to_string());

    let info = reason_info(&reason);

    let system = if let Some(code) = query.system_code {
        sqlx::query(
            r#"SELECT id, code, name, status as "status: SystemStatus" FROM portal_systems WHERE code = $1"#,
        )
        .bind(&code)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("system query failed: {}", e)))?
        .map(|r| SystemSummary {
            id: r.get("id"),
            code: r.get("code"),
            name: r.get("name"),
            status: r.get("status"),
        })
    } else {
        None
    };

    let tenant = if let Some(id) = query.tenant_id {
        sqlx::query(
            r#"SELECT id, code, name, status as "status: TenantStatus" FROM portal_tenants WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant query failed: {}", e)))?
        .map(|r| AccessDeniedTenantSummary {
            id: r.get("id"),
            code: r.get("code"),
            name: r.get("name"),
            status: r.get("status"),
        })
    } else {
        None
    };

    Ok(Json(AccessDeniedContextResponse {
        reason: reason.clone(),
        title: info.0.to_string(),
        description: info.1.to_string(),
        recovery_action: info.2.to_string(),
        return_to,
        system,
        tenant,
    }))
}

fn reason_info(reason: &str) -> (&'static str, &'static str, &'static str) {
    match reason {
        "system_disabled" => ("系统已停用", "目标系统当前处于停用或维护状态，暂时无法访问。", "返回首页或稍后再试"),
        "tenant_disabled" => ("租户已停用", "所属租户当前处于停用状态，无法继续访问。", "联系租户管理员"),
        "session_expired" => ("登录已过期", "你的登录会话已过期或已失效，请重新登录。", "重新登录"),
        "maintenance" => ("系统维护中", "目标系统正在维护，维护期间暂不可用。", "稍后再试"),
        _ => ("暂无访问权限", "你当前的身份未被授予该资源或系统的访问权限。", "申请权限或联系管理员"),
    }
}
