use axum::{
    extract::{ConnectInfo, Extension, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;
use std::net::SocketAddr;
use uuid::Uuid;

use crate::auth::{require_super_admin, CurrentSession};
use crate::error::AppError;
use crate::models::{
    AdminType, AuditResult, GrantStatus, RoleType, SubjectType, SystemStatus, TenantStatus,
    UserStatus,
};
use crate::permissions::calculator::get_effective_context;
use crate::services::audit::{write_audit, AuditPayload};
use crate::services::auth::revoke_all_user_sessions;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/overview", get(overview))
        .route("/api/admin/users", get(list_users).post(create_user))
        .route("/api/admin/users/:user_id", get(get_user).patch(update_user))
        .route("/api/admin/users/:user_id/roles", put(update_user_roles))
        .route("/api/admin/users/:user_id/tenants", put(update_user_tenants))
        .route("/api/admin/users/batch", post(batch_users))
        .route("/api/admin/tenants", get(list_tenants).post(create_tenant))
        .route("/api/admin/tenants/:tenant_id", get(get_tenant).patch(update_tenant))
        .route("/api/admin/tenants/:tenant_id/members", put(update_tenant_members))
        .route("/api/admin/tenants/:tenant_id/systems", put(update_tenant_systems))
        .route("/api/admin/roles", get(list_roles).post(create_role))
        .route("/api/admin/roles/:role_id", get(get_role).patch(update_role).delete(delete_role))
        .route("/api/admin/roles/:role_id/members", put(update_role_members))
        .route("/api/admin/roles/:role_id/permissions", put(update_role_permissions))
        .route("/api/admin/systems", get(list_systems).post(create_system))
        .route("/api/admin/systems/:system_id", get(get_system).patch(update_system))
        .route("/api/admin/systems/:system_id/status", post(update_system_status))
        .route("/api/admin/permissions/matrix", get(permissions_matrix))
        .route("/api/admin/permissions/assignments", put(save_assignments))
        .route("/api/admin/permissions/preview", post(preview_permissions))
        .route("/api/admin/permissions/effective", get(effective_permissions))
        .route("/api/admin/sub-admins", get(list_sub_admins).post(create_sub_admin))
        .route("/api/admin/sub-admins/:grant_id", patch(update_sub_admin))
        .route("/api/admin/sub-admins/scope-options", get(scope_options))
        .route("/api/admin/integrations", get(list_integrations))
        .route("/api/admin/integrations/:system_id", get(get_integration).put(update_integration))
        .route("/api/admin/integrations/:system_id/check", post(check_integration))
        .route("/api/admin/audits", get(list_audits))
        .route("/api/admin/audits/:audit_id", get(get_audit))
        .route("/api/admin/audits/export", post(export_audits))
        .route("/api/permission-requests", post(create_permission_request))
}

// ============== helpers ==============

#[derive(Clone)]
struct AuditCtx {
    request_id: Option<String>,
    actor_user_id: Option<Uuid>,
    ip_address: Option<String>,
    user_agent: Option<String>,
}

fn audit_ctx(
    session: &CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: &HeaderMap,
) -> AuditCtx {
    AuditCtx {
        request_id: Some(request_id),
        actor_user_id: Some(session.user.id),
        ip_address: Some(addr.ip().to_string()),
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    }
}

async fn admin_audit(pool: &sqlx::PgPool, _ctx: &AuditCtx, payload: AuditPayload) {
    let _ = write_audit(pool, payload).await;
}

fn sanitize_audit_value(value: Value) -> Value {
    match value {
        Value::Object(m) => {
            let mut out = serde_json::Map::new();
            for (k, v) in m {
                let lower = k.to_lowercase();
                let masked_keys = ["password", "password_hash", "passwordhash", "token", "secret", "private_key", "privatekey"];
                let new_v = if masked_keys.contains(&lower.as_str()) {
                    if let Value::String(_) = v {
                        Value::String("***".to_string())
                    } else {
                        sanitize_audit_value(v)
                    }
                } else {
                    sanitize_audit_value(v)
                };
                out.insert(k, new_v);
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sanitize_audit_value).collect()),
        other => other,
    }
}

fn opt_sanitize(value: Option<Value>) -> Option<Value> {
    value.map(sanitize_audit_value)
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    20
}

fn pagination_bounds(page: i64, page_size: i64) -> (i64, i64) {
    let page = page.max(1);
    let page_size = page_size.max(1).min(100);
    let offset = (page - 1) * page_size;
    (page_size, offset)
}

fn enum_snake_case<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

// ============== overview ==============

async fn overview(
    State(state): State<AppState>,
    session: CurrentSession,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let since = Utc::now() - chrono::Duration::hours(24);

    let user_total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portal_users WHERE status != 'archived'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("user count failed: {}", e)))?;

    let active_system_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portal_systems WHERE status = 'active'")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("system count failed: {}", e)))?;

    let portal_managed_system_total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portal_systems WHERE portal_managed = true AND auth_enabled = true AND status = 'active'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("portal managed count failed: {}", e)))?;

    let subsystem_entry_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portal_audit_events WHERE action = 'subsystem.enter' AND occurred_at >= $1",
    )
    .bind(since)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("entry count failed: {}", e)))?;

    let high_risk_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portal_audit_events WHERE action IN ('permission.change','sub_admin.change','user.disable','tenant.change') AND occurred_at >= $1 AND result = 'success'",
    )
    .bind(since)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("high risk count failed: {}", e)))?;

    let failed_ticket_exchanges_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portal_audit_events WHERE action = 'subsystem.ticket.exchange.failure' AND occurred_at >= $1",
    )
    .bind(since)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("ticket failure count failed: {}", e)))?;

    let pending_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portal_users WHERE status = 'pending'")
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("pending count failed: {}", e)))?;

    let onboarding_systems: Vec<IdCodeName> = sqlx::query_as(
        "SELECT id, code, name FROM portal_systems WHERE status = 'onboarding' ORDER BY code",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("onboarding systems failed: {}", e)))?;

    let recent_audits: Vec<AuditListItem> = sqlx::query_as(
        r#"SELECT a.id, a.request_id, a.occurred_at, a.actor_user_id, a.action, a.target_type, a.target_id, a.system_id, a.tenant_id, a.result as "result: AuditResult", a.failure_reason,
                u.display_name as actor_name
         FROM portal_audit_events a
         LEFT JOIN portal_users u ON u.id = a.actor_user_id
         ORDER BY a.occurred_at DESC
         LIMIT 10"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("recent audits failed: {}", e)))?;

    let mut todos = Vec::new();
    for s in onboarding_systems {
        todos.push(serde_json::json!({
            "type": "integration_incomplete",
            "title": format!("{} 接入配置未完成", s.name),
            "targetType": "system",
            "targetId": s.id,
        }));
    }
    if pending_users > 0 {
        todos.push(serde_json::json!({
            "type": "pending_users",
            "title": format!("{} 位待激活用户已分配权限", pending_users),
            "targetType": "user",
            "targetId": "",
        }));
    }
    if failed_ticket_exchanges_24h > 0 {
        todos.push(serde_json::json!({
            "type": "ticket_exchange_failure",
            "title": format!("近 24 小时 {} 次凭证校验失败", failed_ticket_exchanges_24h),
            "targetType": "system",
            "targetId": "",
        }));
    }

    let recent: Vec<Value> = recent_audits
        .into_iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "occurredAt": a.occurred_at,
                "actorName": a.actor_name.unwrap_or_else(|| "系统".to_string()),
                "action": a.action,
                "targetType": a.target_type,
                "targetId": a.target_id,
                "result": a.result,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "stats": {
            "userTotal": user_total,
            "activeSystemTotal": active_system_total,
            "portalManagedSystemTotal": portal_managed_system_total,
            "subsystemEntry24h": subsystem_entry_24h,
            "highRiskPermissionChanges24h": high_risk_24h,
        },
        "todos": todos,
        "recentAudits": recent,
    })))
}

// ============== users ==============

#[derive(Deserialize)]
struct UserListQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    keyword: Option<String>,
    status: Option<String>,
    role_code: Option<String>,
    tenant_id: Option<String>,
    system_code: Option<String>,
    organization_path: Option<String>,
}

async fn list_users(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<UserListQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let (limit, offset) = pagination_bounds(q.page, q.page_size);

    let mut where_clauses = vec!["1=1".to_string()];
    if let Some(status) = &q.status {
        where_clauses.push(format!("u.status = '{}'", sanitize_literal(status)));
    }
    if let Some(path) = &q.organization_path {
        where_clauses.push(format!(
            "u.organization_path ILIKE '%{}%'",
            sanitize_like(path)
        ));
    }
    if let Some(keyword) = &q.keyword {
        let kw = sanitize_like(keyword);
        where_clauses.push(format!(
            "(u.username ILIKE '%{0}%' OR u.display_name ILIKE '%{0}%' OR u.email ILIKE '%{0}%' OR u.phone ILIKE '%{0}%')",
            kw
        ));
    }
    if let Some(role_code) = &q.role_code {
        where_clauses.push(format!(
            "EXISTS (SELECT 1 FROM portal_user_roles ur JOIN portal_roles r ON r.id = ur.role_id WHERE ur.user_id = u.id AND ur.tenant_id IS NULL AND r.code = '{}')",
            sanitize_literal(role_code)
        ));
    }
    if let Some(tenant_id) = &q.tenant_id {
        if let Ok(tid) = tenant_id.parse::<Uuid>() {
            where_clauses.push(format!(
                "EXISTS (SELECT 1 FROM portal_tenant_members tm WHERE tm.user_id = u.id AND tm.tenant_id = '{}')",
                tid
            ));
        }
    }
    if q.system_code.is_some() {
        // TODO: implement system_code filter (needs permission/sub-admin/role assignment joins)
    }

    let where_sql = where_clauses.join(" AND ");

    let items: Vec<UserListRow> = sqlx::query_as(&format!(
        r#"SELECT u.id, u.username, u.display_name, u.email, u.phone, u.avatar_url, u.organization_path, u.status as "status", u.default_tenant_id, u.created_at, u.updated_at,
                (SELECT COUNT(*) FROM portal_sessions s WHERE s.user_id = u.id) as session_count
         FROM portal_users u
         WHERE {}
         ORDER BY u.created_at DESC
         LIMIT {} OFFSET {}"#,
        where_sql, limit, offset
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("list users failed: {}", e)))?;

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM portal_users u WHERE {}",
        where_sql
    ))
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("count users failed: {}", e)))?;

    let user_ids: Vec<Uuid> = items.iter().map(|u| u.id).collect();
    let roles = load_user_roles(&state.db, &user_ids).await?;
    let tenants = load_user_tenants(&state.db, &user_ids).await?;

    let items_json: Vec<Value> = items
        .into_iter()
        .map(|u| {
            serde_json::json!({
                "id": u.id,
                "username": u.username,
                "displayName": u.display_name,
                "email": u.email,
                "phone": u.phone,
                "avatarUrl": u.avatar_url,
                "organizationPath": u.organization_path,
                "status": u.status,
                "defaultTenantId": u.default_tenant_id,
                "createdAt": u.created_at,
                "updatedAt": u.updated_at,
                "sessionCount": u.session_count,
                "userRoles": roles.get(&u.id).cloned().unwrap_or_default(),
                "tenantMembers": tenants.get(&u.id).cloned().unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "items": items_json,
        "pagination": {
            "page": q.page.max(1),
            "pageSize": limit,
            "total": total,
            "totalPages": (total as f64 / limit as f64).ceil() as i64,
        }
    })))
}

async fn load_user_roles(
    pool: &sqlx::PgPool,
    user_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<Value>>, AppError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(Uuid, Uuid, String, String, Option<Uuid>)> = sqlx::query_as(
        r#"SELECT ur.user_id, r.id, r.code, r.name, ur.tenant_id
           FROM portal_user_roles ur
           JOIN portal_roles r ON r.id = ur.role_id
           WHERE ur.user_id = ANY($1) AND ur.tenant_id IS NULL"#,
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load roles failed: {}", e)))?;

    let mut map: HashMap<Uuid, Vec<Value>> = HashMap::new();
    for (user_id, id, code, name, _tenant_id) in rows {
        map.entry(user_id).or_default().push(serde_json::json!({
            "id": id,
            "code": code,
            "name": name,
        }));
    }
    Ok(map)
}

async fn load_user_tenants(
    pool: &sqlx::PgPool,
    user_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<Value>>, AppError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(Uuid, Uuid, String, String)> = sqlx::query_as(
        r#"SELECT tm.user_id, t.id, t.code, t.name
           FROM portal_tenant_members tm
           JOIN portal_tenants t ON t.id = tm.tenant_id
           WHERE tm.user_id = ANY($1)"#,
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load tenants failed: {}", e)))?;

    let mut map: HashMap<Uuid, Vec<Value>> = HashMap::new();
    for (user_id, id, code, name) in rows {
        map.entry(user_id).or_default().push(serde_json::json!({
            "id": id,
            "code": code,
            "name": name,
        }));
    }
    Ok(map)
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    display_name: String,
    email: Option<String>,
    phone: Option<String>,
    organization_path: Option<String>,
    #[serde(default = "default_user_status")]
    status: UserStatus,
    password: Option<String>,
    #[serde(default)]
    tenant_ids: Vec<String>,
    #[serde(default)]
    role_ids: Vec<String>,
}

async fn create_user(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let username = req.username.trim();
    let display_name = req.display_name.trim();
    if username.is_empty() || display_name.is_empty() {
        return Err(AppError::ValidationFailed(
            "USERNAME_AND_DISPLAY_NAME_REQUIRED".to_string(),
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

    let password = req.password.as_deref().unwrap_or("portal123");
    let password_hash = crate::services::auth::hash_password(password)?;

    let user_id = Uuid::new_v4();
    let default_tenant_id: Option<Uuid> = req.tenant_ids.first().and_then(|s| s.parse().ok());

    sqlx::query(
        r#"INSERT INTO portal_users (id, username, password_hash, display_name, email, phone, organization_path, status, default_tenant_id, preferences)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, '{}')"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(password_hash)
    .bind(display_name)
    .bind(req.email.as_ref().map(|s| s.trim().to_string()))
    .bind(req.phone.as_ref().map(|s| s.trim().to_string()))
    .bind(req.organization_path.as_ref().map(|s| s.trim().to_string()))
    .bind(req.status.clone())
    .bind(default_tenant_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create user failed: {}", e)))?;

    for tid in &req.tenant_ids {
        if let Ok(tenant_id) = tid.parse::<Uuid>() {
            sqlx::query(
                "INSERT INTO portal_tenant_members (id, tenant_id, user_id, member_status) VALUES ($1, $2, $3, 'active') ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(tenant_id)
            .bind(user_id)
            .execute(&state.db)
            .await
            .ok();
        }
    }

    for rid in &req.role_ids {
        if let Ok(role_id) = rid.parse::<Uuid>() {
            sqlx::query(
                "INSERT INTO portal_user_roles (id, user_id, role_id, tenant_id) VALUES ($1, $2, $3, NULL) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(role_id)
            .execute(&state.db)
            .await
            .ok();
        }
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "user.create".to_string(),
            target_type: "user".to_string(),
            target_id: Some(user_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "id": user_id,
                "username": username,
                "displayName": display_name,
                "status": req.status,
                "tenantIds": req.tenant_ids,
                "roleIds": req.role_ids,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "id": user_id })))
}

async fn get_user(
    State(state): State<AppState>,
    session: CurrentSession,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let user: UserDetailRow = sqlx::query_as(
        r#"SELECT id, username, display_name, email, phone, avatar_url, organization_path, status as "status", default_tenant_id, preferences, last_login_at, created_at, updated_at
           FROM portal_users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get user failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let user_roles: Vec<Value> = sqlx::query_as::<_, UserRoleRow>(
        r#"SELECT ur.id, ur.user_id, ur.tenant_id, r.id as role_id, r.code, r.name, r.role_type as "role_type: RoleType"
           FROM portal_user_roles ur
           JOIN portal_roles r ON r.id = ur.role_id
           WHERE ur.user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("user roles failed: {}", e)))?
    .into_iter()
    .map(|r| {
        serde_json::json!({
            "id": r.id,
            "userId": r.user_id,
            "tenantId": r.tenant_id,
            "role": {
                "id": r.role_id,
                "code": r.code,
                "name": r.name,
                "roleType": r.role_type,
            }
        })
    })
    .collect();

    let tenant_members: Vec<Value> = sqlx::query_as::<_, UserTenantRow>(
        r#"SELECT tm.id, tm.tenant_id, t.code, t.name, t.status as "status: TenantStatus"
           FROM portal_tenant_members tm
           JOIN portal_tenants t ON t.id = tm.tenant_id
           WHERE tm.user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("user tenants failed: {}", e)))?
    .into_iter()
    .map(|t| {
        serde_json::json!({
            "id": t.id,
            "tenantId": t.tenant_id,
            "tenant": {
                "id": t.tenant_id,
                "code": t.code,
                "name": t.name,
                "status": t.status,
            }
        })
    })
    .collect();

    let sub_admin_grants: Vec<Value> = sqlx::query_as::<_, UserGrantRow>(
        r#"SELECT g.id, g.tenant_id, g.system_id, g.admin_type as "admin_type: AdminType", g.scopes, g.status as "status: GrantStatus", g.reason, g.starts_at, g.expires_at,
                s.code as system_code, s.name as system_name
         FROM portal_sub_admin_grants g
         JOIN portal_systems s ON s.id = g.system_id
         WHERE g.user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("user grants failed: {}", e)))?
    .into_iter()
    .map(|g| {
        serde_json::json!({
            "id": g.id,
            "tenantId": g.tenant_id,
            "systemId": g.system_id,
            "system": { "id": g.system_id, "code": g.system_code, "name": g.system_name },
            "adminType": g.admin_type,
            "scopes": g.scopes,
            "status": g.status,
            "reason": g.reason,
            "startsAt": g.starts_at,
            "expiresAt": g.expires_at,
        })
    })
    .collect();

    let permission_assignments: Vec<Value> = load_assignments_for_subject(
        &state.db,
        SubjectType::User,
        user_id,
    )
    .await?;

    let audit_events: Vec<Value> = load_target_audits(&state.db, "user", &user_id.to_string()).await?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "displayName": user.display_name,
            "email": user.email,
            "phone": user.phone,
            "avatarUrl": user.avatar_url,
            "organizationPath": user.organization_path,
            "status": user.status,
            "defaultTenantId": user.default_tenant_id,
            "preferences": user.preferences,
            "lastLoginAt": user.last_login_at,
            "createdAt": user.created_at,
            "updatedAt": user.updated_at,
        },
        "userRoles": user_roles,
        "tenantMembers": tenant_members,
        "subAdminGrants": sub_admin_grants,
        "permissionAssignments": permission_assignments,
        "auditEvents": audit_events,
    })))
}

#[derive(Deserialize)]
struct UpdateUserRequest {
    display_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    organization_path: Option<String>,
    status: Option<UserStatus>,
}

async fn update_user(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let before: UserDetailRow = sqlx::query_as(
        r#"SELECT id, username, display_name, email, phone, avatar_url, organization_path, status as "status", default_tenant_id, preferences, last_login_at, created_at, updated_at
           FROM portal_users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get user failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let mut sets = Vec::new();
    if let Some(v) = req.display_name {
        sets.push(format!("display_name = '{}'", sanitize_literal(&v)));
    }
    if let Some(v) = req.email {
        let val = if v.trim().is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", sanitize_literal(v.trim()))
        };
        sets.push(format!("email = {}", val));
    }
    if let Some(v) = req.phone {
        let val = if v.trim().is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", sanitize_literal(v.trim()))
        };
        sets.push(format!("phone = {}", val));
    }
    if let Some(v) = req.organization_path {
        let val = if v.trim().is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", sanitize_literal(v.trim()))
        };
        sets.push(format!("organization_path = {}", val));
    }
    let status_changed_to_disabled = matches!(req.status, Some(UserStatus::Disabled)) && !matches!(before.status, UserStatus::Disabled);
    if let Some(ref status) = req.status {
        sets.push(format!("status = '{}'", enum_snake_case(status)));
    }

    if sets.is_empty() {
        return Ok(Json(serde_json::json!({ "user": before })));
    }

    sets.push("updated_at = NOW()".to_string());
    let sql = format!(
        "UPDATE portal_users SET {} WHERE id = '{}' RETURNING id, username, display_name, email, phone, avatar_url, organization_path, status, default_tenant_id, preferences, last_login_at, created_at, updated_at",
        sets.join(", "),
        user_id
    );

    let after: UserDetailRow = sqlx::query_as(&sql)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("update user failed: {}", e)))?;

    let mut high_risk = false;
    if status_changed_to_disabled {
        revoke_all_user_sessions(&state.db, user_id).await?;
        high_risk = true;
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: if high_risk { "user.disable".to_string() } else { "user.update".to_string() },
            target_type: "user".to_string(),
            target_id: Some(user_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::to_value(&before).unwrap_or(Value::Null))),
            after_data: opt_sanitize(Some(serde_json::to_value(&after).unwrap_or(Value::Null))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "user": after })))
}

#[derive(Deserialize)]
struct UpdateUserRolesRequest {
    #[serde(default)]
    role_ids: Vec<String>,
}

async fn update_user_roles(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserRolesRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let user_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("user check failed: {}", e)))?;
    if !user_exists {
        return Err(AppError::NotFound);
    }

    let existing: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, role_id FROM portal_user_roles WHERE user_id = $1 AND tenant_id IS NULL",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("existing roles failed: {}", e)))?;

    let existing_ids: Vec<String> = existing.iter().map(|(_, rid)| rid.to_string()).collect();
    let target_ids: Vec<String> = req.role_ids;

    let to_remove: Vec<Uuid> = existing
        .into_iter()
        .filter(|(_, rid)| !target_ids.contains(&rid.to_string()))
        .map(|(id, _)| id)
        .collect();

    let to_add: Vec<Uuid> = target_ids
        .iter()
        .filter(|id| !existing_ids.contains(id))
        .filter_map(|id| id.parse().ok())
        .collect();

    if !to_remove.is_empty() {
        sqlx::query("DELETE FROM portal_user_roles WHERE id = ANY($1)")
            .bind(&to_remove)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("remove roles failed: {}", e)))?;
    }

    for role_id in to_add {
        sqlx::query("INSERT INTO portal_user_roles (id, user_id, role_id, tenant_id) VALUES ($1, $2, $3, NULL) ON CONFLICT DO NOTHING")
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(role_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("add role failed: {}", e)))?;
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "user.roles.update".to_string(),
            target_type: "user".to_string(),
            target_id: Some(user_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::json!({ "roleIds": existing_ids }))),
            after_data: opt_sanitize(Some(serde_json::json!({ "roleIds": target_ids }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "roleIds": target_ids })))
}

#[derive(Deserialize)]
struct UpdateUserTenantsRequest {
    #[serde(default)]
    tenant_ids: Vec<String>,
}

async fn update_user_tenants(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserTenantsRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let user: UserDefaultTenantRow = sqlx::query_as(
        "SELECT id, default_tenant_id FROM portal_users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("user check failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let existing: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, tenant_id FROM portal_tenant_members WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("existing tenants failed: {}", e)))?;

    let existing_ids: Vec<String> = existing.iter().map(|(_, tid)| tid.to_string()).collect();
    let target_ids: Vec<String> = req.tenant_ids;

    let to_remove: Vec<Uuid> = existing
        .into_iter()
        .filter(|(_, tid)| !target_ids.contains(&tid.to_string()))
        .map(|(id, _)| id)
        .collect();

    let to_add: Vec<Uuid> = target_ids
        .iter()
        .filter(|id| !existing_ids.contains(id))
        .filter_map(|id| id.parse().ok())
        .collect();

    if !to_remove.is_empty() {
        sqlx::query("DELETE FROM portal_tenant_members WHERE id = ANY($1)")
            .bind(&to_remove)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("remove tenants failed: {}", e)))?;
    }

    for tenant_id in to_add {
        sqlx::query("INSERT INTO portal_tenant_members (id, tenant_id, user_id, member_status) VALUES ($1, $2, $3, 'active') ON CONFLICT DO NOTHING")
            .bind(Uuid::new_v4())
            .bind(tenant_id)
            .bind(user_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("add tenant failed: {}", e)))?;
    }

    if let Some(first) = target_ids.first() {
        if let Ok(first_tid) = first.parse::<Uuid>() {
            if user.default_tenant_id.map(|d| d != first_tid).unwrap_or(true) {
                sqlx::query("UPDATE portal_users SET default_tenant_id = $1, updated_at = NOW() WHERE id = $2")
                    .bind(first_tid)
                    .bind(user_id)
                    .execute(&state.db)
                    .await
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("update default tenant failed: {}", e)))?;
            }
        }
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "user.tenants.update".to_string(),
            target_type: "user".to_string(),
            target_id: Some(user_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::json!({ "tenantIds": existing_ids }))),
            after_data: opt_sanitize(Some(serde_json::json!({ "tenantIds": target_ids }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "tenantIds": target_ids })))
}

#[derive(Deserialize)]
struct BatchUsersRequest {
    action: String,
    #[serde(default)]
    user_ids: Vec<String>,
    role_id: Option<String>,
    tenant_id: Option<String>,
}

async fn batch_users(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<BatchUsersRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let allowed = ["enable", "disable", "assignRole", "removeRole", "addToTenant", "removeFromTenant"];
    if !allowed.contains(&req.action.as_str()) {
        return Err(AppError::ValidationFailed("INVALID_ACTION".to_string()));
    }
    if req.user_ids.is_empty() {
        return Err(AppError::ValidationFailed("USER_IDS_REQUIRED".to_string()));
    }

    let ids: Vec<Uuid> = req.user_ids.iter().filter_map(|s| s.parse().ok()).collect();
    if ids.is_empty() {
        return Err(AppError::ValidationFailed("NO_MATCHING_USERS".to_string()));
    }

    match req.action.as_str() {
        "enable" => {
            sqlx::query("UPDATE portal_users SET status = 'active', updated_at = NOW() WHERE id = ANY($1)")
                .bind(&ids)
                .execute(&state.db)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("enable users failed: {}", e)))?;
        }
        "disable" => {
            sqlx::query("UPDATE portal_users SET status = 'disabled', updated_at = NOW() WHERE id = ANY($1)")
                .bind(&ids)
                .execute(&state.db)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("disable users failed: {}", e)))?;
            for uid in &ids {
                revoke_all_user_sessions(&state.db, *uid).await.ok();
            }
        }
        "assignRole" => {
            let role_id = req.role_id.as_deref().ok_or_else(|| AppError::ValidationFailed("ROLE_ID_REQUIRED".to_string()))?;
            let role_id = role_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("ROLE_ID_REQUIRED".to_string()))?;
            for uid in &ids {
                sqlx::query("INSERT INTO portal_user_roles (id, user_id, role_id, tenant_id) VALUES ($1, $2, $3, NULL) ON CONFLICT DO NOTHING")
                    .bind(Uuid::new_v4())
                    .bind(uid)
                    .bind(role_id)
                    .execute(&state.db)
                    .await
                    .ok();
            }
        }
        "removeRole" => {
            let role_id = req.role_id.as_deref().ok_or_else(|| AppError::ValidationFailed("ROLE_ID_REQUIRED".to_string()))?;
            let role_id = role_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("ROLE_ID_REQUIRED".to_string()))?;
            sqlx::query("DELETE FROM portal_user_roles WHERE user_id = ANY($1) AND role_id = $2 AND tenant_id IS NULL")
                .bind(&ids)
                .bind(role_id)
                .execute(&state.db)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("remove role failed: {}", e)))?;
        }
        "addToTenant" => {
            let tenant_id = req.tenant_id.as_deref().ok_or_else(|| AppError::ValidationFailed("TENANT_ID_REQUIRED".to_string()))?;
            let tenant_id = tenant_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("TENANT_ID_REQUIRED".to_string()))?;
            for uid in &ids {
                sqlx::query("INSERT INTO portal_tenant_members (id, tenant_id, user_id, member_status) VALUES ($1, $2, $3, 'active') ON CONFLICT DO NOTHING")
                    .bind(Uuid::new_v4())
                    .bind(tenant_id)
                    .bind(uid)
                    .execute(&state.db)
                    .await
                    .ok();
            }
        }
        "removeFromTenant" => {
            let tenant_id = req.tenant_id.as_deref().ok_or_else(|| AppError::ValidationFailed("TENANT_ID_REQUIRED".to_string()))?;
            let tenant_id = tenant_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("TENANT_ID_REQUIRED".to_string()))?;
            sqlx::query("DELETE FROM portal_tenant_members WHERE user_id = ANY($1) AND tenant_id = $2")
                .bind(&ids)
                .bind(tenant_id)
                .execute(&state.db)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("remove from tenant failed: {}", e)))?;
        }
        _ => {}
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: if req.action == "disable" { "user.batch.disable".to_string() } else { "user.batch".to_string() },
            target_type: "user".to_string(),
            target_id: None,
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "action": req.action,
                "affectedCount": ids.len(),
                "affectedIds": ids,
                "roleId": req.role_id,
                "tenantId": req.tenant_id,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "affectedCount": ids.len() })))
}

// ============== tenants ==============

#[derive(Deserialize)]
struct TenantListQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    keyword: Option<String>,
    status: Option<String>,
}

async fn list_tenants(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<TenantListQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let (limit, offset) = pagination_bounds(q.page, q.page_size);

    let mut where_clauses = vec!["1=1".to_string()];
    if let Some(status) = &q.status {
        where_clauses.push(format!("status = '{}'", sanitize_literal(status)));
    }
    if let Some(keyword) = &q.keyword {
        let kw = sanitize_like(keyword);
        where_clauses.push(format!(
            "(code ILIKE '%{0}%' OR name ILIKE '%{0}%')",
            kw
        ));
    }

    let where_sql = where_clauses.join(" AND ");

    let items: Vec<TenantListRow> = sqlx::query_as(&format!(
        r#"SELECT t.id, t.code, t.name, t.status as "status: TenantStatus", t.description, t.created_at, t.updated_at,
                (SELECT COUNT(*) FROM portal_tenant_members tm WHERE tm.tenant_id = t.id) as member_count,
                (SELECT COUNT(*) FROM portal_tenant_systems ts WHERE ts.tenant_id = t.id) as system_count
         FROM portal_tenants t
         WHERE {}
         ORDER BY t.created_at DESC
         LIMIT {} OFFSET {}"#,
        where_sql, limit, offset
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("list tenants failed: {}", e)))?;

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM portal_tenants WHERE {}",
        where_sql
    ))
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("count tenants failed: {}", e)))?;

    let items_json: Vec<Value> = items
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "code": t.code,
                "name": t.name,
                "status": t.status,
                "description": t.description,
                "createdAt": t.created_at,
                "updatedAt": t.updated_at,
                "memberCount": t.member_count,
                "systemCount": t.system_count,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "items": items_json,
        "pagination": {
            "page": q.page.max(1),
            "pageSize": limit,
            "total": total,
            "totalPages": (total as f64 / limit as f64).ceil() as i64,
        }
    })))
}

#[derive(Deserialize)]
struct CreateTenantRequest {
    code: String,
    name: String,
    description: Option<String>,
    #[serde(default = "default_tenant_status")]
    status: TenantStatus,
}

fn default_tenant_status() -> TenantStatus {
    TenantStatus::Active
}

fn default_user_status() -> UserStatus {
    UserStatus::Active
}

async fn create_tenant(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let code = req.code.trim();
    let name = req.name.trim();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::ValidationFailed("CODE_AND_NAME_REQUIRED".to_string()));
    }

    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM portal_tenants WHERE code = $1")
        .bind(code)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("code check failed: {}", e)))?;
    if existing.is_some() {
        return Err(AppError::Conflict("CODE_EXISTS".to_string()));
    }

    let tenant_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO portal_tenants (id, code, name, description, status) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(tenant_id)
    .bind(code)
    .bind(name)
    .bind(req.description.as_ref().map(|s| s.trim().to_string()))
    .bind(req.status.clone())
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create tenant failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "tenant.create".to_string(),
            target_type: "tenant".to_string(),
            target_id: Some(tenant_id.to_string()),
            system_id: None,
            tenant_id: Some(tenant_id),
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "id": tenant_id,
                "code": code,
                "name": name,
                "status": req.status,
                "description": req.description,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "id": tenant_id })))
}

async fn get_tenant(
    State(state): State<AppState>,
    session: CurrentSession,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let tenant: TenantDetailRow = sqlx::query_as(
        r#"SELECT id, code, name, status as "status: TenantStatus", description, created_at, updated_at
           FROM portal_tenants WHERE id = $1"#,
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get tenant failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let members: Vec<Value> = sqlx::query_as::<_, TenantMemberRow>(
        r#"SELECT tm.id, tm.user_id, u.username, u.display_name, u.status as "status"
           FROM portal_tenant_members tm
           JOIN portal_users u ON u.id = tm.user_id
           WHERE tm.tenant_id = $1"#,
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant members failed: {}", e)))?
    .into_iter()
    .map(|m| {
        serde_json::json!({
            "id": m.id,
            "user": {
                "id": m.user_id,
                "username": m.username,
                "displayName": m.display_name,
                "status": m.status,
            }
        })
    })
    .collect();

    let systems: Vec<Value> = sqlx::query_as::<_, TenantSystemRow>(
        r#"SELECT ts.id, ts.system_id, ts.enabled, s.code, s.name, s.status as "status: SystemStatus"
           FROM portal_tenant_systems ts
           JOIN portal_systems s ON s.id = ts.system_id
           WHERE ts.tenant_id = $1"#,
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant systems failed: {}", e)))?
    .into_iter()
    .map(|s| {
        serde_json::json!({
            "id": s.id,
            "systemId": s.system_id,
            "enabled": s.enabled,
            "system": {
                "id": s.system_id,
                "code": s.code,
                "name": s.name,
                "status": s.status,
            }
        })
    })
    .collect();

    let mut admins_by_system: HashMap<String, Vec<Value>> = HashMap::new();
    for s in &systems {
        if let Some(system_id) = s.get("systemId").and_then(|v| v.as_str()).and_then(|s| s.parse::<Uuid>().ok()) {
            let admins: Vec<Value> = sqlx::query_as::<_, GrantUserRow>(
                r#"SELECT g.id, g.user_id, u.username, u.display_name, g.admin_type as "admin_type: AdminType", g.scopes
                   FROM portal_sub_admin_grants g
                   JOIN portal_users u ON u.id = g.user_id
                   WHERE g.tenant_id = $1 AND g.system_id = $2 AND g.status = 'active' AND g.admin_type IN ('system','tenant')"#,
            )
            .bind(tenant_id)
            .bind(system_id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("admins failed: {}", e)))?
            .into_iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "adminType": a.admin_type,
                    "scopes": a.scopes,
                    "user": {
                        "id": a.user_id,
                        "username": a.username,
                        "displayName": a.display_name,
                    }
                })
            })
            .collect();
            admins_by_system.insert(system_id.to_string(), admins);
        }
    }

    let audit_events = load_target_audits(&state.db, "tenant", &tenant_id.to_string()).await?;

    Ok(Json(serde_json::json!({
        "tenant": tenant,
        "members": members,
        "tenantSystems": systems,
        "adminsBySystem": admins_by_system,
        "auditEvents": audit_events,
    })))
}

#[derive(Deserialize)]
struct UpdateTenantRequest {
    name: Option<String>,
    description: Option<String>,
    status: Option<TenantStatus>,
}

async fn update_tenant(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let before: TenantDetailRow = sqlx::query_as(
        r#"SELECT id, code, name, status as "status: TenantStatus", description, created_at, updated_at
           FROM portal_tenants WHERE id = $1"#,
    )
    .bind(tenant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get tenant failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let mut sets = Vec::new();
    if let Some(v) = req.name {
        sets.push(format!("name = '{}'", sanitize_literal(&v)));
    }
    if let Some(v) = req.description {
        let val = if v.trim().is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", sanitize_literal(v.trim()))
        };
        sets.push(format!("description = {}", val));
    }
    let status_changed_to_disabled = matches!(req.status, Some(TenantStatus::Disabled)) && !matches!(before.status, TenantStatus::Disabled);
    if let Some(ref status) = req.status {
        sets.push(format!("status = '{}'", enum_snake_case(status)));
    }
    if sets.is_empty() {
        return Ok(Json(serde_json::json!({ "tenant": before })));
    }

    sets.push("updated_at = NOW()".to_string());
    let sql = format!(
        "UPDATE portal_tenants SET {} WHERE id = '{}' RETURNING id, code, name, status, description, created_at, updated_at",
        sets.join(", "),
        tenant_id
    );

    let after: TenantDetailRow = sqlx::query_as(&sql)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("update tenant failed: {}", e)))?;

    let high_risk = status_changed_to_disabled;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: if high_risk { "tenant.disable".to_string() } else { "tenant.update".to_string() },
            target_type: "tenant".to_string(),
            target_id: Some(tenant_id.to_string()),
            system_id: None,
            tenant_id: Some(tenant_id),
            result: "success",
            before_data: opt_sanitize(Some(serde_json::to_value(&before).unwrap_or(Value::Null))),
            after_data: opt_sanitize(Some(serde_json::to_value(&after).unwrap_or(Value::Null))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "tenant": after })))
}

#[derive(Deserialize)]
struct UpdateTenantMembersRequest {
    #[serde(default)]
    user_ids: Vec<String>,
}

async fn update_tenant_members(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<UpdateTenantMembersRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let tenant_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_tenants WHERE id = $1)")
        .bind(tenant_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant check failed: {}", e)))?;
    if !tenant_exists {
        return Err(AppError::NotFound);
    }

    let existing: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, user_id FROM portal_tenant_members WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("existing members failed: {}", e)))?;

    let existing_ids: Vec<String> = existing.iter().map(|(_, uid)| uid.to_string()).collect();
    let target_ids: Vec<String> = req.user_ids;

    let to_remove: Vec<Uuid> = existing
        .into_iter()
        .filter(|(_, uid)| !target_ids.contains(&uid.to_string()))
        .map(|(id, _)| id)
        .collect();

    let to_add: Vec<Uuid> = target_ids
        .iter()
        .filter(|id| !existing_ids.contains(id))
        .filter_map(|id| id.parse().ok())
        .collect();

    if !to_remove.is_empty() {
        sqlx::query("DELETE FROM portal_tenant_members WHERE id = ANY($1)")
            .bind(&to_remove)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("remove members failed: {}", e)))?;
    }

    for user_id in to_add {
        sqlx::query("INSERT INTO portal_tenant_members (id, tenant_id, user_id, member_status) VALUES ($1, $2, $3, 'active') ON CONFLICT DO NOTHING")
            .bind(Uuid::new_v4())
            .bind(tenant_id)
            .bind(user_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("add member failed: {}", e)))?;
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "tenant.members.update".to_string(),
            target_type: "tenant".to_string(),
            target_id: Some(tenant_id.to_string()),
            system_id: None,
            tenant_id: Some(tenant_id),
            result: "success",
            before_data: opt_sanitize(Some(serde_json::json!({ "userIds": existing_ids }))),
            after_data: opt_sanitize(Some(serde_json::json!({ "userIds": target_ids }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "userIds": target_ids })))
}

#[derive(Deserialize)]
struct UpdateTenantSystemsRequest {
    #[serde(default)]
    system_ids: Vec<String>,
}

async fn update_tenant_systems(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(req): Json<UpdateTenantSystemsRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let tenant_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_tenants WHERE id = $1)")
        .bind(tenant_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant check failed: {}", e)))?;
    if !tenant_exists {
        return Err(AppError::NotFound);
    }

    let existing: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, system_id FROM portal_tenant_systems WHERE tenant_id = $1",
    )
    .bind(tenant_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("existing systems failed: {}", e)))?;

    let existing_ids: Vec<String> = existing.iter().map(|(_, sid)| sid.to_string()).collect();
    let target_ids: Vec<String> = req.system_ids;

    let to_remove: Vec<Uuid> = existing
        .into_iter()
        .filter(|(_, sid)| !target_ids.contains(&sid.to_string()))
        .map(|(id, _)| id)
        .collect();

    let to_add: Vec<Uuid> = target_ids
        .iter()
        .filter(|id| !existing_ids.contains(id))
        .filter_map(|id| id.parse().ok())
        .collect();

    if !to_remove.is_empty() {
        sqlx::query("DELETE FROM portal_tenant_systems WHERE id = ANY($1)")
            .bind(&to_remove)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("remove systems failed: {}", e)))?;
    }

    for system_id in to_add {
        sqlx::query("INSERT INTO portal_tenant_systems (id, tenant_id, system_id, enabled) VALUES ($1, $2, $3, true) ON CONFLICT DO NOTHING")
            .bind(Uuid::new_v4())
            .bind(tenant_id)
            .bind(system_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("add system failed: {}", e)))?;
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "tenant.systems.update".to_string(),
            target_type: "tenant".to_string(),
            target_id: Some(tenant_id.to_string()),
            system_id: None,
            tenant_id: Some(tenant_id),
            result: "success",
            before_data: opt_sanitize(Some(serde_json::json!({ "systemIds": existing_ids }))),
            after_data: opt_sanitize(Some(serde_json::json!({ "systemIds": target_ids }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "systemIds": target_ids })))
}

// ============== roles ==============

#[derive(Deserialize)]
struct RoleListQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    keyword: Option<String>,
    role_type: Option<String>,
}

async fn list_roles(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<RoleListQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let (limit, offset) = pagination_bounds(q.page, q.page_size);

    let mut where_clauses = vec!["1=1".to_string()];
    if let Some(role_type) = &q.role_type {
        where_clauses.push(format!("role_type = '{}'", sanitize_literal(role_type)));
    }
    if let Some(keyword) = &q.keyword {
        let kw = sanitize_like(keyword);
        where_clauses.push(format!(
            "(code ILIKE '%{0}%' OR name ILIKE '%{0}%')",
            kw
        ));
    }

    let where_sql = where_clauses.join(" AND ");

    let items: Vec<RoleListRow> = sqlx::query_as(&format!(
        r#"SELECT r.id, r.code, r.name, r.role_type as "role_type: RoleType", r.description, r.is_builtin, r.created_at, r.updated_at,
                (SELECT COUNT(*) FROM portal_user_roles ur WHERE ur.role_id = r.id) as member_count
         FROM portal_roles r
         WHERE {}
         ORDER BY r.created_at DESC
         LIMIT {} OFFSET {}"#,
        where_sql, limit, offset
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("list roles failed: {}", e)))?;

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM portal_roles WHERE {}",
        where_sql
    ))
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("count roles failed: {}", e)))?;

    let items_json: Vec<Value> = items
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "code": r.code,
                "name": r.name,
                "roleType": r.role_type,
                "description": r.description,
                "isBuiltin": r.is_builtin,
                "createdAt": r.created_at,
                "updatedAt": r.updated_at,
                "memberCount": r.member_count,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "items": items_json,
        "pagination": {
            "page": q.page.max(1),
            "pageSize": limit,
            "total": total,
            "totalPages": (total as f64 / limit as f64).ceil() as i64,
        }
    })))
}

#[derive(Deserialize)]
struct CreateRoleRequest {
    code: String,
    name: String,
    #[serde(default = "default_role_type")]
    role_type: RoleType,
    description: Option<String>,
}

fn default_role_type() -> RoleType {
    RoleType::Custom
}

async fn create_role(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let code = req.code.trim();
    let name = req.name.trim();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::ValidationFailed("CODE_AND_NAME_REQUIRED".to_string()));
    }

    let existing: Option<(Uuid, bool)> = sqlx::query_as("SELECT id, is_builtin FROM portal_roles WHERE code = $1")
        .bind(code)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("code check failed: {}", e)))?;
    if let Some((_, is_builtin)) = existing {
        if is_builtin {
            return Err(AppError::Conflict("BUILTIN_ROLE_EXISTS".to_string()));
        }
        return Err(AppError::Conflict("CODE_EXISTS".to_string()));
    }

    let role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO portal_roles (id, code, name, role_type, description, is_builtin) VALUES ($1, $2, $3, $4, $5, false)",
    )
    .bind(role_id)
    .bind(code)
    .bind(name)
    .bind(req.role_type.clone())
    .bind(req.description.as_ref().map(|s| s.trim().to_string()))
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create role failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "role.create".to_string(),
            target_type: "role".to_string(),
            target_id: Some(role_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "id": role_id,
                "code": code,
                "name": name,
                "roleType": req.role_type,
                "description": req.description,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "id": role_id })))
}

async fn get_role(
    State(state): State<AppState>,
    session: CurrentSession,
    Path(role_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let role: RoleDetailRow = sqlx::query_as(
        r#"SELECT id, code, name, role_type as "role_type: RoleType", description, is_builtin, created_at, updated_at
           FROM portal_roles WHERE id = $1"#,
    )
    .bind(role_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get role failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let members: Vec<Value> = sqlx::query_as::<_, RoleMemberRow>(
        r#"SELECT ur.id, ur.user_id, u.username, u.display_name, u.status as "status"
           FROM portal_user_roles ur
           JOIN portal_users u ON u.id = ur.user_id
           WHERE ur.role_id = $1 AND ur.tenant_id IS NULL"#,
    )
    .bind(role_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("role members failed: {}", e)))?
    .into_iter()
    .map(|m| {
        serde_json::json!({
            "id": m.id,
            "userId": m.user_id,
            "user": {
                "id": m.user_id,
                "username": m.username,
                "displayName": m.display_name,
                "status": m.status,
            }
        })
    })
    .collect();

    let permission_assignments = load_assignments_for_subject(&state.db, SubjectType::Role, role_id).await?;

    let tenant_scope: Vec<String> = {
        let mut scopes: Vec<String> = permission_assignments
            .iter()
            .filter_map(|a| a.get("tenantId").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();
        scopes.sort();
        scopes.dedup();
        scopes
    };

    let audit_events = load_target_audits(&state.db, "role", &role_id.to_string()).await?;

    Ok(Json(serde_json::json!({
        "role": role,
        "userRoles": members,
        "permissionAssignments": permission_assignments,
        "tenantScope": tenant_scope,
        "auditEvents": audit_events,
    })))
}

#[derive(Deserialize)]
struct UpdateRoleRequest {
    name: Option<String>,
    description: Option<String>,
    role_type: Option<RoleType>,
}

async fn update_role(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let before: RoleDetailRow = sqlx::query_as(
        r#"SELECT id, code, name, role_type as "role_type: RoleType", description, is_builtin, created_at, updated_at
           FROM portal_roles WHERE id = $1"#,
    )
    .bind(role_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get role failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let mut sets = Vec::new();
    if let Some(v) = req.name {
        sets.push(format!("name = '{}'", sanitize_literal(&v)));
    }
    if let Some(v) = req.description {
        let val = if v.trim().is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", sanitize_literal(v.trim()))
        };
        sets.push(format!("description = {}", val));
    }
    if let Some(role_type) = req.role_type {
        sets.push(format!("role_type = '{}'", enum_snake_case(&role_type)));
    }
    if sets.is_empty() {
        return Ok(Json(serde_json::json!({ "role": before })));
    }

    sets.push("updated_at = NOW()".to_string());
    let sql = format!(
        "UPDATE portal_roles SET {} WHERE id = '{}' RETURNING id, code, name, role_type, description, is_builtin, created_at, updated_at",
        sets.join(", "),
        role_id
    );

    let after: RoleDetailRow = sqlx::query_as(&sql)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("update role failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "role.update".to_string(),
            target_type: "role".to_string(),
            target_id: Some(role_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::to_value(&before).unwrap_or(Value::Null))),
            after_data: opt_sanitize(Some(serde_json::to_value(&after).unwrap_or(Value::Null))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "role": after })))
}

async fn delete_role(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let role: RoleDeleteCheckRow = sqlx::query_as(
        r#"SELECT id, code, name, role_type as "role_type: RoleType", description, is_builtin, created_at, updated_at,
                (SELECT COUNT(*) FROM portal_user_roles ur WHERE ur.role_id = r.id) as member_count
         FROM portal_roles r WHERE id = $1"#,
    )
    .bind(role_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get role failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    if role.is_builtin {
        return Err(AppError::PermissionDenied);
    }
    if role.member_count > 0 {
        return Err(AppError::Conflict("ROLE_HAS_MEMBERS".to_string()));
    }

    sqlx::query("DELETE FROM portal_roles WHERE id = $1")
        .bind(role_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("delete role failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "role.delete".to_string(),
            target_type: "role".to_string(),
            target_id: Some(role_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::to_value(&role).unwrap_or(Value::Null))),
            after_data: None,
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Deserialize)]
struct UpdateRoleMembersRequest {
    #[serde(default)]
    user_ids: Vec<String>,
}

async fn update_role_members(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
    Json(req): Json<UpdateRoleMembersRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let role_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_roles WHERE id = $1)")
        .bind(role_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("role check failed: {}", e)))?;
    if !role_exists {
        return Err(AppError::NotFound);
    }

    let existing: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, user_id FROM portal_user_roles WHERE role_id = $1 AND tenant_id IS NULL",
    )
    .bind(role_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("existing members failed: {}", e)))?;

    let existing_ids: Vec<String> = existing.iter().map(|(_, uid)| uid.to_string()).collect();
    let target_ids: Vec<String> = req.user_ids;

    let to_remove: Vec<Uuid> = existing
        .into_iter()
        .filter(|(_, uid)| !target_ids.contains(&uid.to_string()))
        .map(|(id, _)| id)
        .collect();

    let to_add: Vec<Uuid> = target_ids
        .iter()
        .filter(|id| !existing_ids.contains(id))
        .filter_map(|id| id.parse().ok())
        .collect();

    if !to_remove.is_empty() {
        sqlx::query("DELETE FROM portal_user_roles WHERE id = ANY($1)")
            .bind(&to_remove)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("remove members failed: {}", e)))?;
    }

    for user_id in to_add {
        sqlx::query("INSERT INTO portal_user_roles (id, user_id, role_id, tenant_id) VALUES ($1, $2, $3, NULL) ON CONFLICT DO NOTHING")
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(role_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("add member failed: {}", e)))?;
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "role.members.update".to_string(),
            target_type: "role".to_string(),
            target_id: Some(role_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::json!({ "userIds": existing_ids }))),
            after_data: opt_sanitize(Some(serde_json::json!({ "userIds": target_ids }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "userIds": target_ids })))
}

#[derive(Deserialize)]
struct UpdateRolePermissionsRequest {
    #[serde(default)]
    assignments: Vec<AssignmentInput>,
}

#[derive(Deserialize, Clone)]
struct AssignmentInput {
    #[serde(default)]
    id: Option<String>,
    #[serde(default = "default_subject_type")]
    subject_type: SubjectType,
    #[serde(default)]
    subject_id: Option<String>,
    #[serde(default)]
    tenant_id: Option<String>,
    system_id: String,
    #[serde(default)]
    visible: bool,
    #[serde(default)]
    accessible: bool,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default)]
    system_roles: Vec<String>,
    #[serde(default)]
    scopes: Vec<AdminScopeInput>,
    source_note: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
}

fn default_subject_type() -> SubjectType {
    SubjectType::User
}

#[derive(Deserialize, Serialize, Clone)]
struct AdminScopeInput {
    scope_type: String,
    scope_code: String,
}

async fn update_role_permissions(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
    Json(req): Json<UpdateRolePermissionsRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let role_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_roles WHERE id = $1)")
        .bind(role_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("role check failed: {}", e)))?;
    if !role_exists {
        return Err(AppError::NotFound);
    }

    let system_ids: Vec<Uuid> = req
        .assignments
        .iter()
        .filter_map(|a| a.system_id.parse().ok())
        .collect();
    if !system_ids.is_empty() {
        let existing_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portal_systems WHERE id = ANY($1)")
            .bind(&system_ids)
            .fetch_one(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("system check failed: {}", e)))?;
        if existing_count != system_ids.len() as i64 {
            return Err(AppError::NotFound);
        }
    }

    let before: Vec<AssignmentStored> = sqlx::query_as(
        r#"SELECT id, subject_type as "subject_type: SubjectType", subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at
           FROM portal_permission_assignments
           WHERE subject_type = 'role' AND subject_id = $1"#,
    )
    .bind(role_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load before failed: {}", e)))?;

    sqlx::query("DELETE FROM portal_permission_assignments WHERE subject_type = 'role' AND subject_id = $1")
        .bind(role_id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("delete assignments failed: {}", e)))?;

    for a in &req.assignments {
        let system_id = a.system_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("SYSTEM_NOT_FOUND".to_string()))?;
        let tenant_id: Option<Uuid> = a.tenant_id.as_deref().and_then(|s| s.parse().ok());
        let scopes = serde_json::to_value(&a.scopes).unwrap_or(Value::Array(vec![]));
        sqlx::query(
            r#"INSERT INTO portal_permission_assignments
               (id, subject_type, subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at, created_by, updated_by)
               VALUES ($1, 'role', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)"#,
        )
        .bind(Uuid::new_v4())
        .bind(role_id)
        .bind(tenant_id)
        .bind(system_id)
        .bind(a.visible)
        .bind(a.accessible)
        .bind(&a.system_roles)
        .bind(&a.permissions)
        .bind(scopes)
        .bind(a.source_note.as_deref())
        .bind(a.starts_at)
        .bind(a.expires_at)
        .bind(session.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("insert assignment failed: {}", e)))?;
    }

    let after_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portal_permission_assignments WHERE subject_type = 'role' AND subject_id = $1",
    )
    .bind(role_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("count after failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "role.permissions.update".to_string(),
            target_type: "role".to_string(),
            target_id: Some(role_id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::json!({ "count": before.len(), "assignments": before }))),
            after_data: opt_sanitize(Some(serde_json::json!({ "count": after_count }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "count": after_count })))
}

// ============== systems ==============

#[derive(Deserialize)]
struct SystemListQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    keyword: Option<String>,
    category: Option<String>,
    status: Option<String>,
}

async fn list_systems(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<SystemListQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let (limit, offset) = pagination_bounds(q.page, q.page_size);

    let mut where_clauses = vec!["1=1".to_string()];
    if let Some(status) = &q.status {
        where_clauses.push(format!("status = '{}'", sanitize_literal(status)));
    }
    if let Some(category) = &q.category {
        where_clauses.push(format!("category ILIKE '%{}%'", sanitize_like(category)));
    }
    if let Some(keyword) = &q.keyword {
        let kw = sanitize_like(keyword);
        where_clauses.push(format!(
            "(code ILIKE '%{0}%' OR name ILIKE '%{0}%')",
            kw
        ));
    }

    let where_sql = where_clauses.join(" AND ");

    let items: Vec<SystemListRow> = sqlx::query_as(&format!(
        r#"SELECT s.id, s.code, s.name, s.description, s.category, s.icon_url, s.entry_url, s.callback_url, s.status as "status: SystemStatus", s.portal_managed, s.auth_enabled, s.supports_sub_admin, s.supported_identity_levels, s.supported_permissions, s.supported_scopes, s.created_at, s.updated_at,
                (SELECT COUNT(*) FROM portal_tenant_systems ts WHERE ts.system_id = s.id) as tenant_count,
                (SELECT COUNT(*) FROM portal_permission_assignments pa WHERE pa.system_id = s.id) as assignment_count
         FROM portal_systems s
         WHERE {}
         ORDER BY s.created_at DESC
         LIMIT {} OFFSET {}"#,
        where_sql, limit, offset
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("list systems failed: {}", e)))?;

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM portal_systems WHERE {}",
        where_sql
    ))
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("count systems failed: {}", e)))?;

    let items_json: Vec<Value> = items
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "code": s.code,
                "name": s.name,
                "description": s.description,
                "category": s.category,
                "iconUrl": s.icon_url,
                "entryUrl": s.entry_url,
                "callbackUrl": s.callback_url,
                "status": s.status,
                "portalManaged": s.portal_managed,
                "authEnabled": s.auth_enabled,
                "supportsSubAdmin": s.supports_sub_admin,
                "supportedIdentityLevels": s.supported_identity_levels,
                "supportedPermissions": s.supported_permissions,
                "supportedScopes": s.supported_scopes,
                "createdAt": s.created_at,
                "updatedAt": s.updated_at,
                "tenantCount": s.tenant_count,
                "assignmentCount": s.assignment_count,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "items": items_json,
        "pagination": {
            "page": q.page.max(1),
            "pageSize": limit,
            "total": total,
            "totalPages": (total as f64 / limit as f64).ceil() as i64,
        }
    })))
}

#[derive(Deserialize)]
struct CreateSystemRequest {
    code: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    icon_url: Option<String>,
    entry_url: String,
    callback_url: Option<String>,
    #[serde(default = "default_system_status")]
    status: SystemStatus,
    #[serde(default)]
    portal_managed: bool,
    #[serde(default)]
    auth_enabled: bool,
    #[serde(default)]
    supports_sub_admin: bool,
    #[serde(default)]
    supported_identity_levels: Vec<String>,
    #[serde(default)]
    supported_permissions: Vec<String>,
    #[serde(default)]
    supported_scopes: Vec<Value>,
    integration_config: Option<IntegrationConfigInput>,
}

fn default_system_status() -> SystemStatus {
    SystemStatus::Active
}

#[derive(Deserialize)]
struct IntegrationConfigInput {
    issuer: Option<String>,
    #[serde(default = "default_auth_mode")]
    auth_mode: crate::models::AuthMode,
    #[serde(default = "default_token_ttl")]
    token_ttl_seconds: i32,
    public_key: Option<String>,
    verify_endpoint: Option<String>,
    #[serde(default)]
    env_template: Value,
}

fn default_auth_mode() -> crate::models::AuthMode {
    crate::models::AuthMode::Jwt
}

fn default_token_ttl() -> i32 {
    300
}

async fn create_system(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CreateSystemRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let code = req.code.trim();
    let name = req.name.trim();
    let entry_url = req.entry_url.trim();
    if code.is_empty() || name.is_empty() || entry_url.is_empty() {
        return Err(AppError::ValidationFailed("CODE_NAME_ENTRY_URL_REQUIRED".to_string()));
    }

    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM portal_systems WHERE code = $1")
        .bind(code)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("code check failed: {}", e)))?;
    if existing.is_some() {
        return Err(AppError::Conflict("CODE_EXISTS".to_string()));
    }

    let system_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO portal_systems
           (id, code, name, description, category, icon_url, entry_url, callback_url, status, portal_managed, auth_enabled, supports_sub_admin, supported_identity_levels, supported_permissions, supported_scopes)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"#,
    )
    .bind(system_id)
    .bind(code)
    .bind(name)
    .bind(req.description.as_ref().map(|s| s.trim().to_string()))
    .bind(req.category.as_ref().map(|s| s.trim().to_string()))
    .bind(req.icon_url.as_ref().map(|s| s.trim().to_string()))
    .bind(entry_url)
    .bind(req.callback_url.as_ref().map(|s| s.trim().to_string()))
    .bind(req.status.clone())
    .bind(req.portal_managed)
    .bind(req.auth_enabled)
    .bind(req.supports_sub_admin)
    .bind(serde_json::to_value(&req.supported_identity_levels).unwrap_or(Value::Array(vec![])))
    .bind(serde_json::to_value(&req.supported_permissions).unwrap_or(Value::Array(vec![])))
    .bind(serde_json::to_value(&req.supported_scopes).unwrap_or(Value::Array(vec![])))
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create system failed: {}", e)))?;

    if let Some(cfg) = req.integration_config {
        sqlx::query(
            r#"INSERT INTO portal_integration_configs
               (system_id, issuer, auth_mode, token_ttl_seconds, public_key, verify_endpoint, env_template)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(system_id)
        .bind(cfg.issuer.as_ref().map(|s| s.trim().to_string()).unwrap_or_else(|| "http://localhost:8080".to_string()))
        .bind(cfg.auth_mode)
        .bind(cfg.token_ttl_seconds)
        .bind(cfg.public_key.as_ref().map(|s| s.trim().to_string()))
        .bind(cfg.verify_endpoint.as_ref().map(|s| s.trim().to_string()))
        .bind(cfg.env_template)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("create integration config failed: {}", e)))?;
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "system.create".to_string(),
            target_type: "system".to_string(),
            target_id: Some(system_id.to_string()),
            system_id: Some(system_id),
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "id": system_id,
                "code": code,
                "name": name,
                "status": req.status,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "id": system_id })))
}

async fn get_system(
    State(state): State<AppState>,
    session: CurrentSession,
    Path(system_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let system: SystemDetailRow = sqlx::query_as(
        r#"SELECT s.id, s.code, s.name, s.description, s.category, s.icon_url, s.entry_url, s.callback_url, s.status as "status: SystemStatus", s.portal_managed, s.auth_enabled, s.supports_sub_admin, s.supported_identity_levels, s.supported_permissions, s.supported_scopes, s.created_at, s.updated_at,
                (SELECT COUNT(*) FROM portal_tenant_systems ts WHERE ts.system_id = s.id) as tenant_count,
                (SELECT COUNT(*) FROM portal_permission_assignments pa WHERE pa.system_id = s.id) as assignment_count,
                (SELECT COUNT(*) FROM portal_sub_admin_grants g WHERE g.system_id = s.id) as grant_count
         FROM portal_systems s WHERE s.id = $1"#,
    )
    .bind(system_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get system failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let integration: Option<IntegrationRow> = sqlx::query_as(
        r#"SELECT system_id, issuer, auth_mode as "auth_mode: crate::models::AuthMode", token_ttl_seconds, public_key, verify_endpoint, env_template, last_check_at, last_check_result
           FROM portal_integration_configs WHERE system_id = $1"#,
    )
    .bind(system_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get integration failed: {}", e)))?;

    let tenant_systems: Vec<Value> = sqlx::query_as::<_, SystemTenantRow>(
        r#"SELECT ts.id, ts.tenant_id, ts.enabled, t.code, t.name, t.status as "status: TenantStatus"
           FROM portal_tenant_systems ts
           JOIN portal_tenants t ON t.id = ts.tenant_id
           WHERE ts.system_id = $1"#,
    )
    .bind(system_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant systems failed: {}", e)))?
    .into_iter()
    .map(|t| {
        serde_json::json!({
            "id": t.id,
            "tenantId": t.tenant_id,
            "enabled": t.enabled,
            "tenant": {
                "id": t.tenant_id,
                "code": t.code,
                "name": t.name,
                "status": t.status,
            }
        })
    })
    .collect();

    let audit_events = load_target_audits(&state.db, "system", &system_id.to_string()).await?;

    Ok(Json(serde_json::json!({
        "system": {
            "id": system.id,
            "code": system.code,
            "name": system.name,
            "description": system.description,
            "category": system.category,
            "iconUrl": system.icon_url,
            "entryUrl": system.entry_url,
            "callbackUrl": system.callback_url,
            "status": system.status,
            "portalManaged": system.portal_managed,
            "authEnabled": system.auth_enabled,
            "supportsSubAdmin": system.supports_sub_admin,
            "supportedIdentityLevels": system.supported_identity_levels,
            "supportedPermissions": system.supported_permissions,
            "supportedScopes": system.supported_scopes,
            "createdAt": system.created_at,
            "updatedAt": system.updated_at,
            "tenantCount": system.tenant_count,
            "assignmentCount": system.assignment_count,
            "grantCount": system.grant_count,
            "integrationConfig": integration,
            "tenantSystems": tenant_systems,
        },
        "auditEvents": audit_events,
    })))
}

#[derive(Deserialize)]
struct UpdateSystemRequest {
    name: Option<String>,
    description: Option<String>,
    category: Option<String>,
    icon_url: Option<String>,
    entry_url: Option<String>,
    callback_url: Option<String>,
    portal_managed: Option<bool>,
    auth_enabled: Option<bool>,
    supports_sub_admin: Option<bool>,
    supported_identity_levels: Option<Vec<String>>,
    supported_permissions: Option<Vec<String>>,
    supported_scopes: Option<Vec<Value>>,
    integration_config: Option<IntegrationConfigInput>,
}

async fn update_system(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(system_id): Path<Uuid>,
    Json(req): Json<UpdateSystemRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let before: SystemDetailRow = sqlx::query_as(
        r#"SELECT s.id, s.code, s.name, s.description, s.category, s.icon_url, s.entry_url, s.callback_url, s.status as "status: SystemStatus", s.portal_managed, s.auth_enabled, s.supports_sub_admin, s.supported_identity_levels, s.supported_permissions, s.supported_scopes, s.created_at, s.updated_at,
                0::bigint as tenant_count, 0::bigint as assignment_count, 0::bigint as grant_count
         FROM portal_systems s WHERE s.id = $1"#,
    )
    .bind(system_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get system failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let mut sets = Vec::new();
    if let Some(v) = req.name {
        sets.push(format!("name = '{}'", sanitize_literal(&v)));
    }
    if let Some(v) = req.description {
        let val = if v.trim().is_empty() { "NULL".to_string() } else { format!("'{}'", sanitize_literal(v.trim())) };
        sets.push(format!("description = {}", val));
    }
    if let Some(v) = req.category {
        let val = if v.trim().is_empty() { "NULL".to_string() } else { format!("'{}'", sanitize_literal(v.trim())) };
        sets.push(format!("category = {}", val));
    }
    if let Some(v) = req.icon_url {
        let val = if v.trim().is_empty() { "NULL".to_string() } else { format!("'{}'", sanitize_literal(v.trim())) };
        sets.push(format!("icon_url = {}", val));
    }
    if let Some(v) = req.entry_url {
        sets.push(format!("entry_url = '{}'", sanitize_literal(&v)));
    }
    if let Some(v) = req.callback_url {
        let val = if v.trim().is_empty() { "NULL".to_string() } else { format!("'{}'", sanitize_literal(v.trim())) };
        sets.push(format!("callback_url = {}", val));
    }
    if let Some(v) = req.portal_managed {
        sets.push(format!("portal_managed = {}", v));
    }
    if let Some(v) = req.auth_enabled {
        sets.push(format!("auth_enabled = {}", v));
    }
    if let Some(v) = req.supports_sub_admin {
        sets.push(format!("supports_sub_admin = {}", v));
    }
    if let Some(v) = req.supported_identity_levels {
        sets.push(format!(
            "supported_identity_levels = '{}'",
            sanitize_literal(&serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()))
        ));
    }
    if let Some(v) = req.supported_permissions {
        sets.push(format!(
            "supported_permissions = '{}'",
            sanitize_literal(&serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()))
        ));
    }
    if let Some(v) = req.supported_scopes {
        sets.push(format!(
            "supported_scopes = '{}'",
            sanitize_literal(&serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()))
        ));
    }

    if !sets.is_empty() {
        sets.push("updated_at = NOW()".to_string());
        let sql = format!(
            "UPDATE portal_systems SET {} WHERE id = '{}' RETURNING id, code, name, description, category, icon_url, entry_url, callback_url, status, portal_managed, auth_enabled, supports_sub_admin, supported_identity_levels, supported_permissions, supported_scopes, created_at, updated_at",
            sets.join(", "),
            system_id
        );
        sqlx::query(&sql)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("update system failed: {}", e)))?;
    }

    if let Some(cfg) = req.integration_config {
        let existing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_integration_configs WHERE system_id = $1)")
            .bind(system_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("integration check failed: {}", e)))?;

        if existing {
            let mut cfg_sets = vec!["updated_at = NOW()".to_string()];
            if let Some(v) = cfg.issuer {
                cfg_sets.push(format!("issuer = '{}'", sanitize_literal(&v)));
            }
            cfg_sets.push(format!("auth_mode = '{}'", enum_snake_case(&cfg.auth_mode)));
            cfg_sets.push(format!("token_ttl_seconds = {}", cfg.token_ttl_seconds));
            if let Some(v) = cfg.public_key {
                cfg_sets.push(format!("public_key = '{}'", sanitize_literal(&v)));
            } else {
                cfg_sets.push("public_key = NULL".to_string());
            }
            if let Some(v) = cfg.verify_endpoint {
                cfg_sets.push(format!("verify_endpoint = '{}'", sanitize_literal(&v)));
            } else {
                cfg_sets.push("verify_endpoint = NULL".to_string());
            }
            cfg_sets.push(format!(
                "env_template = '{}'",
                sanitize_literal(&serde_json::to_string(&cfg.env_template).unwrap_or_else(|_| "{}".to_string()))
            ));
            let sql = format!(
                "UPDATE portal_integration_configs SET {} WHERE system_id = '{}'",
                cfg_sets.join(", "),
                system_id
            );
            sqlx::query(&sql)
                .execute(&state.db)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("update integration failed: {}", e)))?;
        } else {
            sqlx::query(
                r#"INSERT INTO portal_integration_configs
                   (system_id, issuer, auth_mode, token_ttl_seconds, public_key, verify_endpoint, env_template)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            )
            .bind(system_id)
            .bind(cfg.issuer.as_ref().map(|s| s.trim().to_string()).unwrap_or_else(|| "http://localhost:8080".to_string()))
            .bind(cfg.auth_mode)
            .bind(cfg.token_ttl_seconds)
            .bind(cfg.public_key.as_ref().map(|s| s.trim().to_string()))
            .bind(cfg.verify_endpoint.as_ref().map(|s| s.trim().to_string()))
            .bind(cfg.env_template)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("create integration failed: {}", e)))?;
        }
    }

    let after: SystemDetailRow = sqlx::query_as(
        r#"SELECT s.id, s.code, s.name, s.description, s.category, s.icon_url, s.entry_url, s.callback_url, s.status as "status: SystemStatus", s.portal_managed, s.auth_enabled, s.supports_sub_admin, s.supported_identity_levels, s.supported_permissions, s.supported_scopes, s.created_at, s.updated_at,
                0::bigint as tenant_count, 0::bigint as assignment_count, 0::bigint as grant_count
         FROM portal_systems s WHERE s.id = $1"#,
    )
    .bind(system_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get system after failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "system.update".to_string(),
            target_type: "system".to_string(),
            target_id: Some(system_id.to_string()),
            system_id: Some(system_id),
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::to_value(&before).unwrap_or(Value::Null))),
            after_data: opt_sanitize(Some(serde_json::to_value(&after).unwrap_or(Value::Null))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "system": after })))
}

#[derive(Deserialize)]
struct UpdateSystemStatusRequest {
    status: Option<SystemStatus>,
}

async fn update_system_status(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(system_id): Path<Uuid>,
    Json(req): Json<UpdateSystemStatusRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let before: SystemStatusRow = sqlx::query_as(
        r#"SELECT id, code, name, status as "status: SystemStatus" FROM portal_systems WHERE id = $1"#,
    )
    .bind(system_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get system failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let next_status = if let Some(s) = req.status {
        s
    } else {
        let order = [SystemStatus::Active, SystemStatus::Maintenance, SystemStatus::Disabled, SystemStatus::Onboarding];
        let idx = order.iter().position(|x| matches!(x, _ if std::mem::discriminant(x) == std::mem::discriminant(&before.status))).unwrap_or(0);
        order[(idx + 1) % order.len()].clone()
    };

    let after: SystemDetailRow = sqlx::query_as(&format!(
        "UPDATE portal_systems SET status = '{}', updated_at = NOW() WHERE id = '{}' RETURNING id, code, name, description, category, icon_url, entry_url, callback_url, status, portal_managed, auth_enabled, supports_sub_admin, supported_identity_levels, supported_permissions, supported_scopes, created_at, updated_at",
        enum_snake_case(&next_status),
        system_id
    ))
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("update status failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "system.status.change".to_string(),
            target_type: "system".to_string(),
            target_id: Some(system_id.to_string()),
            system_id: Some(system_id),
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(serde_json::to_value(&before).unwrap_or(Value::Null))),
            after_data: opt_sanitize(Some(serde_json::to_value(&after).unwrap_or(Value::Null))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "system": after })))
}

// ============== permissions ==============

#[derive(Deserialize)]
struct MatrixQuery {
    #[serde(default = "default_matrix_view")]
    view: String,
    tenant_id: Option<String>,
}

fn default_matrix_view() -> String {
    "user".to_string()
}

async fn permissions_matrix(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<MatrixQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let view = match q.view.as_str() {
        "user" | "role" | "tenant" | "system" => q.view.clone(),
        _ => "user".to_string(),
    };

    let systems: Vec<IdCodeName> = sqlx::query_as(
        "SELECT id, code, name FROM portal_systems WHERE status != 'disabled' ORDER BY code",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load systems failed: {}", e)))?;

    let tenants: Vec<IdCodeName> = sqlx::query_as(
        "SELECT id, code, name FROM portal_tenants WHERE status = 'active' ORDER BY code",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load tenants failed: {}", e)))?;

    let mut tenant_id = q.tenant_id.as_deref().and_then(|s| s.parse::<Uuid>().ok());
    if (view == "user" || view == "role") && tenant_id.is_none() && !tenants.is_empty() {
        tenant_id = Some(tenants[0].id);
    }

    let system_ids: Vec<Uuid> = systems.iter().map(|s| s.id).collect();

    let (subjects, rows): (Vec<Value>, Vec<Value>) = match view.as_str() {
        "user" => {
            let users: Vec<SubjectRow> = sqlx::query_as(
                r#"SELECT id, username as code, display_name as name, status as "status" FROM portal_users WHERE status != 'archived' ORDER BY username"#,
            )
            .fetch_all(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("load users failed: {}", e)))?;

            let assignments = load_matrix_assignments(&state.db, SubjectType::User, &system_ids, tenant_id).await?;
            build_matrix_rows(&users, &systems, &assignments, "user", tenant_id)
        }
        "role" => {
            let roles: Vec<SubjectRow> = sqlx::query_as(
                r#"SELECT id, code, name, NULL::"UserStatus" as "status" FROM portal_roles ORDER BY code"#,
            )
            .fetch_all(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("load roles failed: {}", e)))?;

            let assignments = load_matrix_assignments(&state.db, SubjectType::Role, &system_ids, tenant_id).await?;
            build_matrix_rows(&roles, &systems, &assignments, "role", tenant_id)
        }
        "tenant" => {
            let active_tenants: Vec<SubjectRow> = tenants.iter().map(|t| SubjectRow {
                id: t.id,
                code: t.code.clone(),
                name: t.name.clone(),
                status: None,
            }).collect();

            let assignments = load_matrix_assignments(&state.db, SubjectType::Tenant, &system_ids, None).await?;
            build_matrix_rows(&active_tenants, &systems, &assignments, "tenant", None)
        }
        "system" => {
            let system_subjects: Vec<SubjectRow> = systems.iter().map(|s| SubjectRow {
                id: s.id,
                code: s.code.clone(),
                name: s.name.clone(),
                status: None,
            }).collect();

            let assignments = load_matrix_assignments(&state.db, SubjectType::Tenant, &system_ids, None).await?;
            let mut rows = Vec::new();
            let mut subjects = Vec::new();
            for s in &system_subjects {
                subjects.push(serde_json::json!({
                    "id": s.id,
                    "code": s.code,
                    "name": s.name,
                }));
                let mut cells = Vec::new();
                for t in &tenants {
                    let key = format!("{}:{}:{}:{}", "tenant", t.id, t.id, s.id);
                    let cell = assignments.get(&key).cloned().unwrap_or_else(|| serde_json::json!({
                        "visible": false,
                        "accessible": false,
                        "permissions": [],
                        "systemRoles": [],
                        "scopes": [],
                    }));
                    cells.push(cell);
                }
                rows.push(serde_json::json!({
                    "subject": {
                        "id": s.id,
                        "code": s.code,
                        "name": s.name,
                    },
                    "cells": cells,
                }));
            }
            (subjects, rows)
        }
        _ => (vec![], vec![]),
    };

    Ok(Json(serde_json::json!({
        "view": view,
        "tenantId": tenant_id,
        "systems": systems,
        "tenants": tenants,
        "subjects": subjects,
        "rows": rows,
    })))
}

async fn load_matrix_assignments(
    pool: &sqlx::PgPool,
    subject_type: SubjectType,
    system_ids: &[Uuid],
    tenant_id: Option<Uuid>,
) -> Result<HashMap<String, Value>, AppError> {
    if system_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<AssignmentStored> = if let Some(tid) = tenant_id {
        sqlx::query_as(
            r#"SELECT id, subject_type as "subject_type: SubjectType", subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at
               FROM portal_permission_assignments
               WHERE subject_type = $1 AND system_id = ANY($2) AND tenant_id = $3"#,
        )
        .bind(subject_type)
        .bind(system_ids)
        .bind(tid)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("load matrix assignments failed: {}", e)))?
    } else {
        sqlx::query_as(
            r#"SELECT id, subject_type as "subject_type: SubjectType", subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at
               FROM portal_permission_assignments
               WHERE subject_type = $1 AND system_id = ANY($2)"#,
        )
        .bind(subject_type)
        .bind(system_ids)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("load matrix assignments failed: {}", e)))?
    };

    let mut map = HashMap::new();
    for a in rows {
        let key = format!(
            "{}:{}:{}:{}",
            enum_snake_case(&a.subject_type),
            a.subject_id,
            a.tenant_id.map(|t| t.to_string()).unwrap_or_else(|| "null".to_string()),
            a.system_id
        );
        map.insert(
            key,
            serde_json::json!({
                "assignmentId": a.id,
                "visible": a.visible,
                "accessible": a.accessible,
                "permissions": a.permissions,
                "systemRoles": a.system_roles,
                "scopes": a.scopes,
            }),
        );
    }
    Ok(map)
}

fn build_matrix_rows(
    subjects: &[SubjectRow],
    systems: &[IdCodeName],
    assignments: &HashMap<String, Value>,
    subject_type: &str,
    tenant_id: Option<Uuid>,
) -> (Vec<Value>, Vec<Value>) {
    let tenant_key = tenant_id.map(|t| t.to_string()).unwrap_or_else(|| "null".to_string());
    let mut subjects_json = Vec::new();
    let mut rows = Vec::new();
    for s in subjects {
        let subject_json = if s.status.is_some() {
            serde_json::json!({
                "id": s.id,
                "code": s.code,
                "name": s.name,
                "status": s.status,
            })
        } else {
            serde_json::json!({
                "id": s.id,
                "code": s.code,
                "name": s.name,
            })
        };
        subjects_json.push(subject_json.clone());

        let mut cells = Vec::new();
        for sys in systems {
            let key = format!("{}:{}:{}:{}", subject_type, s.id, tenant_key, sys.id);
            let cell = assignments
                .get(&key)
                .cloned()
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "visible": false,
                        "accessible": false,
                        "permissions": [],
                        "systemRoles": [],
                        "scopes": [],
                    })
                });
            cells.push(cell);
        }
        rows.push(serde_json::json!({
            "subject": subject_json,
            "cells": cells,
        }));
    }
    (subjects_json, rows)
}

#[derive(Deserialize)]
struct SaveAssignmentsRequest {
    #[serde(default)]
    assignments: Vec<AssignmentInput>,
}

async fn save_assignments(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<SaveAssignmentsRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let mut before_map = serde_json::Map::new();
    let mut after_map = serde_json::Map::new();

    for a in &req.assignments {
        let subject_type = a.subject_type.clone();
        let system_id = a.system_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("SYSTEM_NOT_FOUND".to_string()))?;
        let subject_id: Uuid = a
            .subject_id
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(system_id);
        let tenant_id: Option<Uuid> = a.tenant_id.as_deref().and_then(|s| s.parse().ok());

        let existing: Option<AssignmentStored> = if let Some(id_str) = &a.id {
            if let Ok(id) = id_str.parse::<Uuid>() {
                sqlx::query_as(
                    r#"SELECT id, subject_type as "subject_type: SubjectType", subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at
                       FROM portal_permission_assignments WHERE id = $1"#,
                )
                .bind(id)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("load assignment failed: {}", e)))?
            } else {
                None
            }
        } else {
            sqlx::query_as(
                r#"SELECT id, subject_type as "subject_type: SubjectType", subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at
                   FROM portal_permission_assignments
                   WHERE subject_type = $1 AND subject_id = $2 AND tenant_id IS NOT DISTINCT FROM $3 AND system_id = $4"#,
            )
            .bind(subject_type.clone())
            .bind(subject_id)
            .bind(tenant_id)
            .bind(system_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("load assignment failed: {}", e)))?
        };

        let key = format!(
            "{}:{}:{}:{}:{}",
            enum_snake_case(&subject_type),
            subject_id,
            tenant_id.map(|t| t.to_string()).unwrap_or_else(|| "null".to_string()),
            system_id,
            a.id.as_deref().unwrap_or("new")
        );

        let scopes = serde_json::to_value(&a.scopes).unwrap_or(Value::Array(vec![]));
        if let Some(existing) = existing {
            before_map.insert(
                key.clone(),
                serde_json::json!({
                    "id": existing.id,
                    "visible": existing.visible,
                    "accessible": existing.accessible,
                    "permissions": existing.permissions,
                    "systemRoles": existing.system_roles,
                    "scopes": existing.scopes,
                    "sourceNote": existing.source_note,
                    "startsAt": existing.starts_at,
                    "expiresAt": existing.expires_at,
                }),
            );
            sqlx::query(
                r#"UPDATE portal_permission_assignments
                   SET visible = $1, accessible = $2, system_roles = $3, permissions = $4, scopes = $5, source_note = $6, starts_at = $7, expires_at = $8, updated_by = $9, updated_at = NOW()
                   WHERE id = $10"#,
            )
            .bind(a.visible)
            .bind(a.accessible)
            .bind(&a.system_roles)
            .bind(&a.permissions)
            .bind(&scopes)
            .bind(a.source_note.as_deref())
            .bind(a.starts_at)
            .bind(a.expires_at)
            .bind(session.user.id)
            .bind(existing.id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("update assignment failed: {}", e)))?;

            after_map.insert(
                key,
                serde_json::json!({
                    "id": existing.id,
                    "visible": a.visible,
                    "accessible": a.accessible,
                    "permissions": a.permissions,
                    "systemRoles": a.system_roles,
                    "scopes": scopes,
                    "sourceNote": a.source_note,
                    "startsAt": a.starts_at,
                    "expiresAt": a.expires_at,
                }),
            );
        } else {
            before_map.insert(key.clone(), Value::Null);
            let new_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO portal_permission_assignments
                   (id, subject_type, subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at, created_by, updated_by)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14)"#,
            )
            .bind(new_id)
            .bind(subject_type.clone())
            .bind(subject_id)
            .bind(tenant_id)
            .bind(system_id)
            .bind(a.visible)
            .bind(a.accessible)
            .bind(&a.system_roles)
            .bind(&a.permissions)
            .bind(&scopes)
            .bind(a.source_note.as_deref())
            .bind(a.starts_at)
            .bind(a.expires_at)
            .bind(session.user.id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("insert assignment failed: {}", e)))?;

            after_map.insert(
                key,
                serde_json::json!({
                    "id": new_id,
                    "visible": a.visible,
                    "accessible": a.accessible,
                    "permissions": a.permissions,
                    "systemRoles": a.system_roles,
                    "scopes": scopes,
                    "sourceNote": a.source_note,
                    "startsAt": a.starts_at,
                    "expiresAt": a.expires_at,
                }),
            );
        }
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "permission.change".to_string(),
            target_type: "permission_assignment_batch".to_string(),
            target_id: None,
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: opt_sanitize(Some(Value::Object(before_map))),
            after_data: opt_sanitize(Some(Value::Object(after_map))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "saved": req.assignments.len() })))
}

async fn preview_permissions(
    State(state): State<AppState>,
    session: CurrentSession,
    Json(req): Json<SaveAssignmentsRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let mut diffs = Vec::new();

    for a in &req.assignments {
        let subject_type = a.subject_type.clone();
        let system_id = a.system_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("SYSTEM_NOT_FOUND".to_string()))?;
        let subject_id: Uuid = a
            .subject_id
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(system_id);
        let tenant_id: Option<Uuid> = a.tenant_id.as_deref().and_then(|s| s.parse().ok());

        let key = format!(
            "{}:{}:{}:{}:{}",
            enum_snake_case(&subject_type),
            subject_id,
            tenant_id.map(|t| t.to_string()).unwrap_or_else(|| "null".to_string()),
            system_id,
            a.id.as_deref().unwrap_or("new")
        );

        let existing: Option<AssignmentStored> = sqlx::query_as(
            r#"SELECT id, subject_type as "subject_type: SubjectType", subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at
               FROM portal_permission_assignments
               WHERE subject_type = $1 AND subject_id = $2 AND tenant_id IS NOT DISTINCT FROM $3 AND system_id = $4"#,
        )
        .bind(subject_type.clone())
        .bind(subject_id)
        .bind(tenant_id)
        .bind(system_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("preview load failed: {}", e)))?;

        let after = serde_json::json!({
            "visible": a.visible,
            "accessible": a.accessible,
            "permissions": a.permissions,
            "systemRoles": a.system_roles,
            "scopes": a.scopes,
            "sourceNote": a.source_note,
            "startsAt": a.starts_at,
            "expiresAt": a.expires_at,
        });

        if let Some(existing) = existing {
            let before = serde_json::json!({
                "visible": existing.visible,
                "accessible": existing.accessible,
                "permissions": existing.permissions,
                "systemRoles": existing.system_roles,
                "scopes": existing.scopes,
                "sourceNote": existing.source_note,
                "startsAt": existing.starts_at,
                "expiresAt": existing.expires_at,
            });
            let changed = serde_json::to_string(&before).unwrap_or_default() != serde_json::to_string(&after).unwrap_or_default();
            diffs.push(serde_json::json!({
                "key": key,
                "action": if changed { "update" } else { "unchanged" },
                "before": before,
                "after": after,
            }));
        } else {
            diffs.push(serde_json::json!({
                "key": key,
                "action": "create",
                "before": Value::Null,
                "after": after,
            }));
        }
    }

    Ok(Json(serde_json::json!({ "diffs": diffs })))
}

#[derive(Deserialize)]
struct EffectiveQuery {
    user_id: String,
    tenant_id: Option<String>,
    system_code: String,
}

async fn effective_permissions(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<EffectiveQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let user_id = q.user_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("MISSING_PARAMS".to_string()))?;

    let system: IdCodeName = sqlx::query_as(
        "SELECT id, code, name FROM portal_systems WHERE code = $1",
    )
    .bind(&q.system_code)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("system query failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let tenant_id: Uuid = if let Some(tid) = q.tenant_id.as_deref().and_then(|s| s.parse::<Uuid>().ok()) {
        tid
    } else {
        // Resolve default tenant for the user
        let default_tid: Option<Uuid> = sqlx::query_scalar("SELECT default_tenant_id FROM portal_users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("load default tenant failed: {}", e)))?;

        if let Some(tid) = default_tid {
            tid
        } else {
            sqlx::query_scalar(
                r#"SELECT tm.tenant_id FROM portal_tenant_members tm
                   JOIN portal_tenants t ON t.id = tm.tenant_id
                   WHERE tm.user_id = $1 AND tm.member_status = 'active' AND t.status = 'active'
                   ORDER BY tm.joined_at LIMIT 1"#,
            )
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("load first tenant failed: {}", e)))?
            .ok_or_else(|| AppError::ValidationFailed("NO_ACTIVE_TENANT".to_string()))?
        }
    };

    let access = get_effective_context(&state.db, user_id, system.id, tenant_id)
        .await?
        .map(|ctx| crate::models::SystemAccess {
            system_id: ctx.system_id,
            system_code: ctx.system_code,
            name: ctx.name,
            description: ctx.description,
            icon_url: ctx.icon_url,
            entry_url: ctx.entry_url,
            category: ctx.category,
            status: ctx.status,
            visible: ctx.visible,
            accessible: ctx.accessible,
            system_roles: ctx.system_roles,
            permissions: ctx.permissions,
            scopes: ctx.admin_scopes,
            identity_label: ctx.identity_label,
            is_sub_admin: ctx.is_sub_admin,
            admin_type: ctx.admin_type.and_then(|s| match s.as_str() {
                "system" => Some(crate::models::AdminType::System),
                "tenant" => Some(crate::models::AdminType::Tenant),
                "module" => Some(crate::models::AdminType::Module),
                "resource" => Some(crate::models::AdminType::Resource),
                "organization" => Some(crate::models::AdminType::Organization),
                _ => None,
            }),
        })
        .unwrap_or_else(|| crate::models::SystemAccess {
            system_id: system.id,
            system_code: system.code.clone(),
            name: system.name.clone(),
            description: None,
            icon_url: None,
            entry_url: String::new(),
            category: None,
            status: crate::models::SystemStatus::Active,
            visible: false,
            accessible: false,
            system_roles: vec![],
            permissions: vec![],
            scopes: vec![],
            identity_label: String::new(),
            is_sub_admin: false,
            admin_type: None,
        });

    let tenant_name: Option<String> = sqlx::query_scalar("SELECT name FROM portal_tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant name failed: {}", e)))?;

    let identity_label = derive_identity_label(&access.system_roles, &access.scopes);

    Ok(Json(serde_json::json!({
        "userId": user_id,
        "tenantId": tenant_id,
        "tenantName": tenant_name,
        "systemId": system.id,
        "systemCode": system.code,
        "systemName": system.name,
        "visible": access.visible,
        "accessible": access.accessible,
        "systemRoles": access.system_roles,
        "permissions": access.permissions,
        "adminScopes": access.scopes,
        "identityLabel": identity_label,
    })))
}

fn derive_identity_label(system_roles: &[String], admin_scopes: &[crate::models::AdminScope]) -> String {
    if system_roles.contains(&"super-admin".to_string()) {
        return "超级管理员".to_string();
    }
    if system_roles.contains(&"tenant-admin".to_string()) {
        return "租户管理员".to_string();
    }
    if system_roles.contains(&"module-admin".to_string()) || !admin_scopes.is_empty() {
        return "模块管理员".to_string();
    }
    if system_roles.contains(&"admin".to_string()) {
        return "管理员".to_string();
    }
    "普通用户".to_string()
}

// ============== sub-admins ==============

#[derive(Deserialize)]
struct SubAdminListQuery {
    status: Option<String>,
    system_code: Option<String>,
    tenant_id: Option<String>,
    user_id: Option<String>,
    keyword: Option<String>,
}

async fn list_sub_admins(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<SubAdminListQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let mut where_clauses = vec!["1=1".to_string()];
    if let Some(status) = &q.status {
        where_clauses.push(format!("g.status = '{}'", sanitize_literal(status)));
    }
    if let Some(system_code) = &q.system_code {
        where_clauses.push(format!("s.code = '{}'", sanitize_literal(system_code)));
    }
    if let Some(tenant_id) = &q.tenant_id {
        if let Ok(tid) = tenant_id.parse::<Uuid>() {
            where_clauses.push(format!("g.tenant_id = '{}'", tid));
        }
    }
    if let Some(user_id) = &q.user_id {
        if let Ok(uid) = user_id.parse::<Uuid>() {
            where_clauses.push(format!("g.user_id = '{}'", uid));
        }
    }
    if let Some(keyword) = &q.keyword {
        let kw = sanitize_like(keyword);
        where_clauses.push(format!(
            "(u.username ILIKE '%{0}%' OR u.display_name ILIKE '%{0}%')",
            kw
        ));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"SELECT g.id, g.user_id, u.username, u.display_name, g.tenant_id, t.name as tenant_name, g.system_id, s.code as system_code, s.name as system_name,
                g.admin_type as "admin_type: AdminType", g.scopes, g.status as "status: GrantStatus", g.reason, g.starts_at, g.expires_at, g.created_at, g.updated_at
         FROM portal_sub_admin_grants g
         JOIN portal_users u ON u.id = g.user_id
         JOIN portal_systems s ON s.id = g.system_id
         LEFT JOIN portal_tenants t ON t.id = g.tenant_id
         WHERE {}
         ORDER BY g.created_at DESC"#,
        where_sql
    );

    let grants: Vec<GrantListRow> = sqlx::query_as(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("list grants failed: {}", e)))?;

    let items_json: Vec<Value> = grants
        .into_iter()
        .map(|g| {
            serde_json::json!({
                "id": g.id,
                "userId": g.user_id,
                "username": g.username,
                "displayName": g.display_name,
                "tenantId": g.tenant_id,
                "tenantName": g.tenant_name,
                "systemId": g.system_id,
                "systemCode": g.system_code,
                "systemName": g.system_name,
                "adminType": g.admin_type,
                "scopes": g.scopes,
                "status": g.status,
                "reason": g.reason,
                "startsAt": g.starts_at,
                "expiresAt": g.expires_at,
                "createdAt": g.created_at,
                "updatedAt": g.updated_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "items": items_json })))
}

#[derive(Deserialize)]
struct CreateSubAdminRequest {
    user_id: String,
    tenant_id: Option<String>,
    system_id: String,
    #[serde(default = "default_admin_type")]
    admin_type: AdminType,
    #[serde(default)]
    scopes: Vec<AdminScopeInput>,
    #[serde(default = "default_grant_status")]
    status: GrantStatus,
    reason: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
}

fn default_grant_status() -> GrantStatus {
    GrantStatus::Active
}

fn default_admin_type() -> AdminType {
    AdminType::System
}

async fn create_sub_admin(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CreateSubAdminRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let user_id = req.user_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("USER_NOT_FOUND".to_string()))?;
    let system_id = req.system_id.parse::<Uuid>().map_err(|_| AppError::ValidationFailed("SYSTEM_NOT_FOUND".to_string()))?;
    let tenant_id = req.tenant_id.as_deref().and_then(|s| s.parse::<Uuid>().ok());

    let user_status: Option<UserStatus> = sqlx::query_scalar(
        r#"SELECT status as "status" FROM portal_users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("user check failed: {}", e)))?;
    if !matches!(user_status, Some(UserStatus::Active)) {
        return Err(AppError::ValidationFailed("USER_NOT_FOUND".to_string()));
    }

    let system_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_systems WHERE id = $1)")
        .bind(system_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("system check failed: {}", e)))?;
    if !system_exists {
        return Err(AppError::ValidationFailed("SYSTEM_NOT_FOUND".to_string()));
    }

    if let Some(tid) = tenant_id {
        let member_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM portal_tenant_members WHERE tenant_id = $1 AND user_id = $2 AND member_status = 'active')",
        )
        .bind(tid)
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("member check failed: {}", e)))?;
        if !member_active {
            return Err(AppError::ValidationFailed("TENANT_MEMBER_NOT_FOUND".to_string()));
        }
    }

    let grant_id = Uuid::new_v4();
    let scopes = serde_json::to_value(&req.scopes).unwrap_or(Value::Array(vec![]));
    let starts_at = req.starts_at.unwrap_or_else(Utc::now);

    sqlx::query(
        r#"INSERT INTO portal_sub_admin_grants
           (id, user_id, tenant_id, system_id, admin_type, scopes, status, reason, starts_at, expires_at, created_by, updated_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)"#,
    )
    .bind(grant_id)
    .bind(user_id)
    .bind(tenant_id)
    .bind(system_id)
    .bind(req.admin_type.clone())
    .bind(scopes)
    .bind(req.status.clone())
    .bind(req.reason.as_deref())
    .bind(starts_at)
    .bind(req.expires_at)
    .bind(session.user.id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create grant failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "sub_admin.create".to_string(),
            target_type: "sub_admin_grant".to_string(),
            target_id: Some(grant_id.to_string()),
            system_id: Some(system_id),
            tenant_id,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "id": grant_id,
                "userId": user_id,
                "tenantId": tenant_id,
                "systemId": system_id,
                "adminType": req.admin_type,
                "scopes": req.scopes,
                "status": req.status,
                "reason": req.reason,
                "startsAt": starts_at,
                "expiresAt": req.expires_at,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "id": grant_id })))
}

#[derive(Deserialize)]
struct UpdateSubAdminRequest {
    admin_type: Option<AdminType>,
    #[serde(default)]
    scopes: Option<Vec<AdminScopeInput>>,
    status: Option<GrantStatus>,
    reason: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    revoke_active_tickets: bool,
}

async fn update_sub_admin(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(grant_id): Path<Uuid>,
    Json(req): Json<UpdateSubAdminRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let grant: GrantUpdateRow = sqlx::query_as(
        r#"SELECT id, user_id, tenant_id, system_id, admin_type as "admin_type: AdminType", scopes, status as "status: GrantStatus", reason, starts_at, expires_at
           FROM portal_sub_admin_grants WHERE id = $1"#,
    )
    .bind(grant_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get grant failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let before = serde_json::json!({
        "adminType": grant.admin_type,
        "scopes": grant.scopes,
        "status": grant.status,
        "reason": grant.reason,
        "startsAt": grant.starts_at,
        "expiresAt": grant.expires_at,
    });

    let mut sets = Vec::new();
    if let Some(v) = req.admin_type {
        sets.push(format!("admin_type = '{}'", enum_snake_case(&v)));
    }
    if let Some(v) = req.scopes {
        sets.push(format!(
            "scopes = '{}'",
            sanitize_literal(&serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()))
        ));
    }
    let status_provided = req.status.is_some();
    if let Some(ref v) = req.status {
        sets.push(format!("status = '{}'", enum_snake_case(v)));
    }
    if let Some(v) = req.reason {
        sets.push(format!("reason = '{}'", sanitize_literal(&v)));
    }
    if let Some(v) = req.starts_at {
        sets.push(format!("starts_at = '{}'", v.to_rfc3339()));
    }
    if let Some(v) = req.expires_at {
        sets.push(format!("expires_at = '{}'", v.to_rfc3339()));
    } else if req.expires_at.is_none() && status_provided {
        sets.push("expires_at = NULL".to_string());
    }

    sets.push("updated_at = NOW()".to_string());
    sets.push(format!("updated_by = '{}'", session.user.id));

    let sql = format!(
        "UPDATE portal_sub_admin_grants SET {} WHERE id = '{}' RETURNING id, user_id, tenant_id, system_id, admin_type, scopes, status, reason, starts_at, expires_at, created_by, updated_by, created_at, updated_at",
        sets.join(", "),
        grant_id
    );

    let updated: GrantDetailRow = sqlx::query_as(&sql)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("update grant failed: {}", e)))?;

    let mut revoked_tickets = 0i64;
    if req.revoke_active_tickets {
        revoked_tickets = sqlx::query(
            "UPDATE portal_subsystem_tickets SET consumed_at = NOW() WHERE user_id = $1 AND system_id = $2 AND consumed_at IS NULL AND expires_at > NOW() RETURNING 1",
        )
        .bind(grant.user_id)
        .bind(grant.system_id)
        .fetch_all(&state.db)
        .await
        .map(|rows: Vec<sqlx::postgres::PgRow>| rows.len() as i64)
        .unwrap_or(0);
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "sub_admin.change".to_string(),
            target_type: "sub_admin_grant".to_string(),
            target_id: Some(grant_id.to_string()),
            system_id: Some(grant.system_id),
            tenant_id: grant.tenant_id,
            result: "success",
            before_data: opt_sanitize(Some(before)),
            after_data: opt_sanitize(Some(serde_json::json!({
                "adminType": updated.admin_type,
                "scopes": updated.scopes,
                "status": updated.status,
                "reason": updated.reason,
                "startsAt": updated.starts_at,
                "expiresAt": updated.expires_at,
                "revokedTickets": revoked_tickets,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({
        "id": grant_id,
        "revokedTickets": revoked_tickets,
    })))
}

#[derive(Deserialize)]
struct ScopeOptionsQuery {
    system_code: String,
}

async fn scope_options(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<ScopeOptionsQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let system: SystemScopeRow = sqlx::query_as(
        r#"SELECT id, code, supports_sub_admin, supported_scopes FROM portal_systems WHERE code = $1"#,
    )
    .bind(&q.system_code)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("system query failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let scopes: Vec<Value> = serde_json::from_value(system.supported_scopes).unwrap_or_default();

    Ok(Json(serde_json::json!({
        "systemId": system.id,
        "systemCode": system.code,
        "supportsSubAdmin": system.supports_sub_admin,
        "scopes": scopes,
    })))
}

// ============== integrations ==============

async fn list_integrations(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<SystemListQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let mut where_clauses = vec!["portal_managed = true".to_string()];
    if let Some(status) = &q.status {
        where_clauses.push(format!("status = '{}'", sanitize_literal(status)));
    }
    if let Some(keyword) = &q.keyword {
        let kw = sanitize_like(keyword);
        where_clauses.push(format!(
            "(code ILIKE '%{0}%' OR name ILIKE '%{0}%')",
            kw
        ));
    }

    let where_sql = where_clauses.join(" AND ");

    let items: Vec<IntegrationListRow> = sqlx::query_as(&format!(
        r#"SELECT s.id, s.code, s.name, s.description, s.category, s.status as "status: SystemStatus", s.entry_url, s.callback_url, s.auth_enabled, s.supports_sub_admin,
                c.issuer, c.auth_mode as "auth_mode: crate::models::AuthMode", c.token_ttl_seconds, c.verify_endpoint, c.last_check_at, c.last_check_result
         FROM portal_systems s
         LEFT JOIN portal_integration_configs c ON c.system_id = s.id
         WHERE {}
         ORDER BY s.code"#,
        where_sql
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("list integrations failed: {}", e)))?;

    let items_json: Vec<Value> = items
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "code": s.code,
                "name": s.name,
                "description": s.description,
                "category": s.category,
                "status": s.status,
                "entryUrl": s.entry_url,
                "callbackUrl": s.callback_url,
                "authEnabled": s.auth_enabled,
                "supportsSubAdmin": s.supports_sub_admin,
                "integration": s.issuer.map(|issuer| serde_json::json!({
                    "issuer": issuer,
                    "authMode": s.auth_mode,
                    "tokenTtlSeconds": s.token_ttl_seconds,
                    "verifyEndpoint": s.verify_endpoint,
                    "lastCheckAt": s.last_check_at,
                    "lastCheckResult": s.last_check_result,
                })),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "items": items_json })))
}

async fn get_integration(
    State(state): State<AppState>,
    session: CurrentSession,
    Path(system_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let system: SystemDetailRow = sqlx::query_as(
        r#"SELECT s.id, s.code, s.name, s.description, s.category, s.icon_url, s.entry_url, s.callback_url, s.status as "status: SystemStatus", s.portal_managed, s.auth_enabled, s.supports_sub_admin, s.supported_identity_levels, s.supported_permissions, s.supported_scopes, s.created_at, s.updated_at,
                0::bigint as tenant_count, 0::bigint as assignment_count, 0::bigint as grant_count
         FROM portal_systems s WHERE s.id = $1"#,
    )
    .bind(system_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get system failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let integration: Option<IntegrationRow> = sqlx::query_as(
        r#"SELECT system_id, issuer, auth_mode as "auth_mode: crate::models::AuthMode", token_ttl_seconds, public_key, verify_endpoint, env_template, last_check_at, last_check_result
           FROM portal_integration_configs WHERE system_id = $1"#,
    )
    .bind(system_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get integration failed: {}", e)))?;

    let env_template = integration.as_ref().map(|i| i.env_template.clone()).unwrap_or(Value::Object(serde_json::Map::new()));
    let env_example = env_template
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or("")))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "id": system.id,
        "code": system.code,
        "name": system.name,
        "description": system.description,
        "category": system.category,
        "status": system.status,
        "entryUrl": system.entry_url,
        "callbackUrl": system.callback_url,
        "authEnabled": system.auth_enabled,
        "supportsSubAdmin": system.supports_sub_admin,
        "supportedIdentityLevels": system.supported_identity_levels,
        "supportedPermissions": system.supported_permissions,
        "supportedScopes": system.supported_scopes,
        "integration": integration.map(|i| serde_json::json!({
            "issuer": i.issuer,
            "authMode": i.auth_mode,
            "tokenTtlSeconds": i.token_ttl_seconds,
            "publicKey": i.public_key,
            "verifyEndpoint": i.verify_endpoint,
            "envTemplate": i.env_template,
            "envExample": env_example,
            "lastCheckAt": i.last_check_at,
            "lastCheckResult": i.last_check_result,
        })),
    })))
}

#[derive(Deserialize)]
struct UpdateIntegrationRequest {
    issuer: Option<String>,
    auth_mode: Option<crate::models::AuthMode>,
    token_ttl_seconds: Option<i32>,
    public_key: Option<String>,
    verify_endpoint: Option<String>,
    env_template: Option<Value>,
    callback_url: Option<String>,
}

async fn update_integration(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(system_id): Path<Uuid>,
    Json(req): Json<UpdateIntegrationRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let system_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_systems WHERE id = $1)")
        .bind(system_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("system check failed: {}", e)))?;
    if !system_exists {
        return Err(AppError::NotFound);
    }

    if let Some(callback_url) = &req.callback_url {
        sqlx::query("UPDATE portal_systems SET callback_url = $1, updated_at = NOW() WHERE id = $2")
            .bind(callback_url.trim())
            .bind(system_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("update callback failed: {}", e)))?;
    }

    let existing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_integration_configs WHERE system_id = $1)")
        .bind(system_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("integration check failed: {}", e)))?;

    if existing {
        let mut sets = vec!["updated_at = NOW()".to_string()];
        if let Some(v) = &req.issuer {
            sets.push(format!("issuer = '{}'", sanitize_literal(v)));
        }
        if let Some(ref v) = req.auth_mode {
            sets.push(format!("auth_mode = '{}'", enum_snake_case(v)));
        }
        if let Some(v) = req.token_ttl_seconds {
            sets.push(format!("token_ttl_seconds = {}", v));
        }
        if let Some(v) = &req.public_key {
            sets.push(format!("public_key = '{}'", sanitize_literal(v)));
        } else if req.public_key.is_some() {
            sets.push("public_key = NULL".to_string());
        }
        if let Some(v) = &req.verify_endpoint {
            sets.push(format!("verify_endpoint = '{}'", sanitize_literal(v)));
        } else if req.verify_endpoint.is_some() {
            sets.push("verify_endpoint = NULL".to_string());
        }
        if let Some(v) = &req.env_template {
            sets.push(format!(
                "env_template = '{}'",
                sanitize_literal(&serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()))
            ));
        }
        let sql = format!(
            "UPDATE portal_integration_configs SET {} WHERE system_id = '{}'",
            sets.join(", "),
            system_id
        );
        sqlx::query(&sql)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("update integration failed: {}", e)))?;
    } else {
        sqlx::query(
            r#"INSERT INTO portal_integration_configs
               (system_id, issuer, auth_mode, token_ttl_seconds, public_key, verify_endpoint, env_template)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(system_id)
        .bind(req.issuer.as_ref().map(|s| s.trim().to_string()).unwrap_or_else(|| "http://localhost:8080".to_string()))
        .bind(req.auth_mode.clone().unwrap_or(crate::models::AuthMode::AuthorizationCode))
        .bind(req.token_ttl_seconds.unwrap_or(300))
        .bind(req.public_key.as_ref().map(|s| s.trim().to_string()))
        .bind(req.verify_endpoint.as_ref().map(|s| s.trim().to_string()))
        .bind(req.env_template.clone().unwrap_or(Value::Object(serde_json::Map::new())))
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("create integration failed: {}", e)))?;
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "integration.update".to_string(),
            target_type: "system".to_string(),
            target_id: Some(system_id.to_string()),
            system_id: Some(system_id),
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "systemId": system_id,
                "issuer": req.issuer,
                "authMode": req.auth_mode,
                "tokenTtlSeconds": req.token_ttl_seconds,
                "publicKey": req.public_key,
                "verifyEndpoint": req.verify_endpoint,
                "envTemplate": req.env_template,
                "callbackUrl": req.callback_url,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "id": system_id })))
}

#[derive(Deserialize)]
struct CheckIntegrationRequest {
    system_code: Option<String>,
    callback_url: Option<String>,
    auth_mode: Option<crate::models::AuthMode>,
}

async fn check_integration(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(system_id): Path<Uuid>,
    Json(req): Json<CheckIntegrationRequest>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let system: SystemCheckRow = sqlx::query_as(
        r#"SELECT s.id, s.code, s.callback_url, c.auth_mode as "auth_mode: crate::models::AuthMode"
           FROM portal_systems s
           LEFT JOIN portal_integration_configs c ON c.system_id = s.id
           WHERE s.id = $1"#,
    )
    .bind(system_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get system failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    let code_to_check = req.system_code.as_deref().unwrap_or(&system.code);
    let callback_url = req.callback_url.as_deref().or(system.callback_url.as_deref());
    let auth_mode = req.auth_mode.unwrap_or(system.auth_mode.unwrap_or(crate::models::AuthMode::AuthorizationCode));

    let system_code_matches = code_to_check == system.code;
    let callback_url_valid = callback_url.map(|u| url::Url::parse(u).is_ok()).unwrap_or(false);
    let auth_mode_supported = matches!(auth_mode, crate::models::AuthMode::Jwt | crate::models::AuthMode::AuthorizationCode);

    let passed = system_code_matches && callback_url_valid && auth_mode_supported;
    let checks = serde_json::json!({
        "systemCodeMatches": system_code_matches,
        "callbackUrlValid": callback_url_valid,
        "authModeSupported": auth_mode_supported,
    });

    let result = serde_json::json!({
        "passed": passed,
        "systemCode": code_to_check,
        "callbackUrl": callback_url,
        "authMode": auth_mode,
        "checks": checks,
        "message": if passed { "接入检查通过" } else { "接入检查未通过" },
    });

    sqlx::query(
        "UPDATE portal_integration_configs SET last_check_at = NOW(), last_check_result = $1 WHERE system_id = $2",
    )
    .bind(result.clone())
    .bind(system_id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("update check result failed: {}", e)))?;

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "integration.check".to_string(),
            target_type: "system".to_string(),
            target_id: Some(system_id.to_string()),
            system_id: Some(system_id),
            tenant_id: None,
            result: if passed { "success" } else { "failure" },
            before_data: None,
            after_data: opt_sanitize(Some(result.clone())),
            failure_reason: if passed { None } else { Some(checks.to_string()) },
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(result))
}

// ============== audits ==============

#[derive(Deserialize)]
struct AuditListQuery {
    occurred_at_from: Option<DateTime<Utc>>,
    occurred_at_to: Option<DateTime<Utc>>,
    actor_user_id: Option<String>,
    action: Option<String>,
    target_type: Option<String>,
    system_id: Option<String>,
    tenant_id: Option<String>,
    result: Option<String>,
    #[serde(default)]
    skip: i64,
    #[serde(default = "default_take")]
    take: i64,
}

fn default_take() -> i64 {
    50
}

async fn list_audits(
    State(state): State<AppState>,
    session: CurrentSession,
    Query(q): Query<AuditListQuery>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let mut where_clauses = vec!["1=1".to_string()];
    if let Some(from) = q.occurred_at_from {
        where_clauses.push(format!("a.occurred_at >= '{}'", from.to_rfc3339()));
    }
    if let Some(to) = q.occurred_at_to {
        where_clauses.push(format!("a.occurred_at <= '{}'", to.to_rfc3339()));
    }
    if let Some(actor) = &q.actor_user_id {
        if let Ok(uid) = actor.parse::<Uuid>() {
            where_clauses.push(format!("a.actor_user_id = '{}'", uid));
        }
    }
    if let Some(action) = &q.action {
        where_clauses.push(format!("a.action ILIKE '%{}%'", sanitize_like(action)));
    }
    if let Some(target_type) = &q.target_type {
        where_clauses.push(format!("a.target_type = '{}'", sanitize_literal(target_type)));
    }
    if let Some(system_id) = &q.system_id {
        if let Ok(sid) = system_id.parse::<Uuid>() {
            where_clauses.push(format!("a.system_id = '{}'", sid));
        }
    }
    if let Some(tenant_id) = &q.tenant_id {
        if let Ok(tid) = tenant_id.parse::<Uuid>() {
            where_clauses.push(format!("a.tenant_id = '{}'", tid));
        }
    }
    if let Some(result) = &q.result {
        where_clauses.push(format!("a.result = '{}'", sanitize_literal(result)));
    }

    let where_sql = where_clauses.join(" AND ");
    let take = q.take.max(1).min(200);

    let sql = format!(
        r#"SELECT a.id, a.request_id, a.occurred_at, a.actor_user_id, a.action, a.target_type, a.target_id, a.system_id, a.tenant_id, a.result as "result: AuditResult", a.failure_reason,
                u.display_name as actor_name, u.username as actor_username,
                s.name as system_name, t.name as tenant_name
         FROM portal_audit_events a
         LEFT JOIN portal_users u ON u.id = a.actor_user_id
         LEFT JOIN portal_systems s ON s.id = a.system_id
         LEFT JOIN portal_tenants t ON t.id = a.tenant_id
         WHERE {}
         ORDER BY a.occurred_at DESC
         LIMIT {} OFFSET {}"#,
        where_sql, take, q.skip
    );

    let list: Vec<AuditListItem> = sqlx::query_as(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("list audits failed: {}", e)))?;

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM portal_audit_events a WHERE {}",
        where_sql
    ))
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("count audits failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "list": list,
        "total": total,
        "skip": q.skip,
        "take": take,
    })))
}

async fn get_audit(
    State(state): State<AppState>,
    session: CurrentSession,
    Path(audit_id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let audit: AuditDetailRow = sqlx::query_as(
        r#"SELECT a.id, a.request_id, a.occurred_at, a.actor_user_id, a.action, a.target_type, a.target_id, a.system_id, a.tenant_id, a.result as "result: AuditResult", a.before_data, a.after_data, a.failure_reason, a.ip_address, a.user_agent,
                u.display_name as actor_name, u.username as actor_username,
                s.name as system_name, t.name as tenant_name
         FROM portal_audit_events a
         LEFT JOIN portal_users u ON u.id = a.actor_user_id
         LEFT JOIN portal_systems s ON s.id = a.system_id
         LEFT JOIN portal_tenants t ON t.id = a.tenant_id
         WHERE a.id = $1"#,
    )
    .bind(audit_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("get audit failed: {}", e)))?
    .ok_or(AppError::NotFound)?;

    Ok(Json(serde_json::json!({
        "id": audit.id,
        "requestId": audit.request_id,
        "occurredAt": audit.occurred_at,
        "actorUserId": audit.actor_user_id,
        "actorName": audit.actor_name.or(audit.actor_username).unwrap_or_else(|| "系统".to_string()),
        "action": audit.action,
        "targetType": audit.target_type,
        "targetId": audit.target_id,
        "systemId": audit.system_id,
        "systemName": audit.system_name,
        "tenantId": audit.tenant_id,
        "tenantName": audit.tenant_name,
        "result": audit.result,
        "beforeData": audit.before_data,
        "afterData": audit.after_data,
        "failureReason": audit.failure_reason,
        "ipAddress": audit.ip_address,
        "userAgent": audit.user_agent,
    })))
}

#[derive(Deserialize, Serialize)]
struct ExportAuditsRequest {
    occurred_at_from: Option<DateTime<Utc>>,
    occurred_at_to: Option<DateTime<Utc>>,
    actor_user_id: Option<String>,
    action: Option<String>,
    target_type: Option<String>,
    system_id: Option<String>,
    tenant_id: Option<String>,
    result: Option<String>,
}

async fn export_audits(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExportAuditsRequest>,
) -> Result<Response, AppError> {
    require_super_admin(&crate::auth::load_current_user(&state, session.user.id).await?)?;

    let mut where_clauses = vec!["1=1".to_string()];
    if let Some(from) = req.occurred_at_from {
        where_clauses.push(format!("a.occurred_at >= '{}'", from.to_rfc3339()));
    }
    if let Some(to) = req.occurred_at_to {
        where_clauses.push(format!("a.occurred_at <= '{}'", to.to_rfc3339()));
    }
    if let Some(actor) = &req.actor_user_id {
        if let Ok(uid) = actor.parse::<Uuid>() {
            where_clauses.push(format!("a.actor_user_id = '{}'", uid));
        }
    }
    if let Some(action) = &req.action {
        where_clauses.push(format!("a.action ILIKE '%{}%'", sanitize_like(action)));
    }
    if let Some(target_type) = &req.target_type {
        where_clauses.push(format!("a.target_type = '{}'", sanitize_literal(target_type)));
    }
    if let Some(system_id) = &req.system_id {
        if let Ok(sid) = system_id.parse::<Uuid>() {
            where_clauses.push(format!("a.system_id = '{}'", sid));
        }
    }
    if let Some(tenant_id) = &req.tenant_id {
        if let Ok(tid) = tenant_id.parse::<Uuid>() {
            where_clauses.push(format!("a.tenant_id = '{}'", tid));
        }
    }
    if let Some(result) = &req.result {
        where_clauses.push(format!("a.result = '{}'", sanitize_literal(result)));
    }

    let where_sql = where_clauses.join(" AND ");
    let sql = format!(
        r#"SELECT a.occurred_at, a.request_id,
                COALESCE(u.display_name, u.username, '系统') as actor_name,
                a.action, a.target_type, a.target_id, s.name as system_name, t.name as tenant_name, a.result as "result: AuditResult", a.failure_reason
         FROM portal_audit_events a
         LEFT JOIN portal_users u ON u.id = a.actor_user_id
         LEFT JOIN portal_systems s ON s.id = a.system_id
         LEFT JOIN portal_tenants t ON t.id = a.tenant_id
         WHERE {}
         ORDER BY a.occurred_at DESC"#,
        where_sql
    );

    let list: Vec<AuditExportRow> = sqlx::query_as(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("export audits failed: {}", e)))?;

    let count = list.len();
    let mut csv = String::from("\u{FEFF}");
    csv.push_str("时间,请求ID,操作人,操作,目标类型,目标ID,系统,租户,结果,失败原因\n");
    for a in &list {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            escape_csv(&a.occurred_at.to_rfc3339()),
            escape_csv(&a.request_id.clone().unwrap_or_default()),
            escape_csv(&a.actor_name),
            escape_csv(&a.action),
            escape_csv(&a.target_type),
            escape_csv(&a.target_id.clone().unwrap_or_default()),
            escape_csv(&a.system_name.clone().unwrap_or_default()),
            escape_csv(&a.tenant_name.clone().unwrap_or_default()),
            escape_csv(&enum_snake_case(&a.result)),
            escape_csv(&a.failure_reason.clone().unwrap_or_default()),
        ));
    }

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "audit.export".to_string(),
            target_type: "audit_event_batch".to_string(),
            target_id: None,
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "count": count,
                "filters": req,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"audits_{}.csv\"", Utc::now().timestamp_millis()),
            ),
        ],
        csv,
    )
        .into_response())
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

// ============== permission requests ==============

#[derive(Deserialize)]
struct PermissionRequest {
    #[serde(default = "default_reason")]
    reason: String,
    system_code: Option<String>,
    tenant_id: Option<String>,
    #[serde(default = "default_return_to")]
    return_to: String,
    message: Option<String>,
}

fn default_reason() -> String {
    "no_permission".to_string()
}
fn default_return_to() -> String {
    "/".to_string()
}

async fn create_permission_request(
    State(state): State<AppState>,
    session: CurrentSession,
    Extension(request_id): Extension<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<PermissionRequest>,
) -> Result<Json<Value>, AppError> {
    let system_id = if let Some(code) = &req.system_code {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM portal_systems WHERE code = $1")
            .bind(code)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("system query failed: {}", e)))?
    } else {
        None
    };

    let tenant_id = req.tenant_id.as_deref().and_then(|s| s.parse::<Uuid>().ok());

    let ctx = audit_ctx(&session, Extension(request_id), ConnectInfo(addr), &headers);
    admin_audit(
        &state.db,
        &ctx,
        AuditPayload {
            request_id: ctx.request_id.clone(),
            actor_user_id: ctx.actor_user_id,
            action: "permission.request".to_string(),
            target_type: "permission_request".to_string(),
            target_id: None,
            system_id,
            tenant_id,
            result: "success",
            before_data: None,
            after_data: opt_sanitize(Some(serde_json::json!({
                "reason": req.reason,
                "systemCode": req.system_code,
                "tenantId": req.tenant_id,
                "returnTo": req.return_to,
                "message": req.message,
            }))),
            failure_reason: None,
            ip_address: ctx.ip_address.clone(),
            user_agent: ctx.user_agent.clone(),
        },
    )
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ============== shared helpers ==============

async fn load_assignments_for_subject(
    pool: &sqlx::PgPool,
    subject_type: SubjectType,
    subject_id: Uuid,
) -> Result<Vec<Value>, AppError> {
    let rows: Vec<AssignmentStored> = sqlx::query_as(
        r#"SELECT id, subject_type as "subject_type: SubjectType", subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions, scopes, source_note, starts_at, expires_at
           FROM portal_permission_assignments
           WHERE subject_type = $1 AND subject_id = $2"#,
    )
    .bind(subject_type)
    .bind(subject_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load assignments failed: {}", e)))?;

    let mut result = Vec::new();
    for a in rows {
        let system_name: Option<String> = sqlx::query_scalar("SELECT name FROM portal_systems WHERE id = $1")
            .bind(a.system_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("system name failed: {}", e)))?;
        let tenant: Option<(String, String)> = if let Some(tid) = a.tenant_id {
            sqlx::query_as("SELECT code, name FROM portal_tenants WHERE id = $1")
                .bind(tid)
                .fetch_optional(pool)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant name failed: {}", e)))?
        } else {
            None
        };

        result.push(serde_json::json!({
            "id": a.id,
            "subjectType": a.subject_type,
            "subjectId": a.subject_id,
            "tenantId": a.tenant_id,
            "systemId": a.system_id,
            "system": { "id": a.system_id, "name": system_name },
            "tenant": tenant.map(|(code, name)| serde_json::json!({ "id": a.tenant_id, "code": code, "name": name })),
            "visible": a.visible,
            "accessible": a.accessible,
            "systemRoles": a.system_roles,
            "permissions": a.permissions,
            "scopes": a.scopes,
            "sourceNote": a.source_note,
            "startsAt": a.starts_at,
            "expiresAt": a.expires_at,
        }));
    }
    Ok(result)
}

async fn load_target_audits(
    pool: &sqlx::PgPool,
    target_type: &str,
    target_id: &str,
) -> Result<Vec<Value>, AppError> {
    let rows: Vec<AuditListItem> = sqlx::query_as(
        r#"SELECT a.id, a.request_id, a.occurred_at, a.actor_user_id, a.action, a.target_type, a.target_id, a.system_id, a.tenant_id, a.result as "result: AuditResult", a.failure_reason,
                u.display_name as actor_name
         FROM portal_audit_events a
         LEFT JOIN portal_users u ON u.id = a.actor_user_id
         WHERE (a.target_id = $1 AND a.target_type = $2) OR a.actor_user_id = $3::uuid
         ORDER BY a.occurred_at DESC
         LIMIT 20"#,
    )
    .bind(target_id)
    .bind(target_type)
    .bind(target_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load audits failed: {}", e)))?;

    Ok(rows.into_iter().map(|a| serde_json::to_value(a).unwrap_or(Value::Null)).collect())
}

fn sanitize_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn sanitize_like(value: &str) -> String {
    value.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_").replace('\'', "''")
}

// ============== row types ==============

#[derive(sqlx::FromRow, Serialize)]
struct IdCodeName {
    id: Uuid,
    code: String,
    name: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct AuditListItem {
    id: Uuid,
    request_id: Option<String>,
    occurred_at: DateTime<Utc>,
    actor_user_id: Option<Uuid>,
    action: String,
    target_type: String,
    target_id: Option<String>,
    system_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    #[sqlx(rename = "result")]
    #[serde(rename = "result")]
    result: AuditResult,
    failure_reason: Option<String>,
    #[sqlx(default)]
    actor_name: Option<String>,
}

#[derive(sqlx::FromRow, Serialize)]
struct AuditDetailRow {
    id: Uuid,
    request_id: Option<String>,
    occurred_at: DateTime<Utc>,
    actor_user_id: Option<Uuid>,
    action: String,
    target_type: String,
    target_id: Option<String>,
    system_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    #[sqlx(rename = "result")]
    result: AuditResult,
    before_data: Option<Value>,
    after_data: Option<Value>,
    failure_reason: Option<String>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    actor_name: Option<String>,
    actor_username: Option<String>,
    system_name: Option<String>,
    tenant_name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct AuditExportRow {
    occurred_at: DateTime<Utc>,
    request_id: Option<String>,
    actor_name: String,
    action: String,
    target_type: String,
    target_id: Option<String>,
    system_name: Option<String>,
    tenant_name: Option<String>,
    #[sqlx(rename = "result")]
    result: AuditResult,
    failure_reason: Option<String>,
}

#[derive(sqlx::FromRow, Serialize)]
struct UserListRow {
    id: Uuid,
    username: String,
    display_name: String,
    email: Option<String>,
    phone: Option<String>,
    avatar_url: Option<String>,
    organization_path: Option<String>,
    #[sqlx(rename = "status")]
    status: UserStatus,
    default_tenant_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    session_count: i64,
}

#[derive(sqlx::FromRow, Serialize)]
struct UserDetailRow {
    id: Uuid,
    username: String,
    display_name: String,
    email: Option<String>,
    phone: Option<String>,
    avatar_url: Option<String>,
    organization_path: Option<String>,
    #[sqlx(rename = "status")]
    status: UserStatus,
    default_tenant_id: Option<Uuid>,
    preferences: Value,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct UserDefaultTenantRow {
    id: Uuid,
    default_tenant_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct UserRoleRow {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    role_id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "role_type")]
    role_type: RoleType,
}

#[derive(sqlx::FromRow)]
struct UserTenantRow {
    id: Uuid,
    tenant_id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "status")]
    status: TenantStatus,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct UserGrantRow {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    system_id: Uuid,
    #[sqlx(rename = "admin_type")]
    admin_type: AdminType,
    scopes: Value,
    #[sqlx(rename = "status")]
    status: GrantStatus,
    reason: Option<String>,
    starts_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    system_code: String,
    system_name: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct TenantListRow {
    id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "status")]
    status: TenantStatus,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    member_count: i64,
    system_count: i64,
}

#[derive(sqlx::FromRow, Serialize)]
struct TenantDetailRow {
    id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "status")]
    status: TenantStatus,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct TenantMemberRow {
    id: Uuid,
    user_id: Uuid,
    username: String,
    display_name: String,
    #[sqlx(rename = "status")]
    status: UserStatus,
}

#[derive(sqlx::FromRow)]
struct TenantSystemRow {
    id: Uuid,
    system_id: Uuid,
    enabled: bool,
    code: String,
    name: String,
    #[sqlx(rename = "status")]
    status: TenantStatus,
}

#[derive(sqlx::FromRow)]
struct GrantUserRow {
    id: Uuid,
    user_id: Uuid,
    username: String,
    display_name: String,
    #[sqlx(rename = "admin_type")]
    admin_type: AdminType,
    scopes: Value,
}

#[derive(sqlx::FromRow, Serialize)]
struct RoleListRow {
    id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "role_type")]
    role_type: RoleType,
    description: Option<String>,
    is_builtin: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    member_count: i64,
}

#[derive(sqlx::FromRow, Serialize)]
struct RoleDetailRow {
    id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "role_type")]
    role_type: RoleType,
    description: Option<String>,
    is_builtin: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize)]
struct RoleDeleteCheckRow {
    id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "role_type")]
    role_type: RoleType,
    description: Option<String>,
    is_builtin: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    member_count: i64,
}

#[derive(sqlx::FromRow)]
struct RoleMemberRow {
    id: Uuid,
    user_id: Uuid,
    username: String,
    display_name: String,
    #[sqlx(rename = "status")]
    status: UserStatus,
}

#[derive(sqlx::FromRow, Serialize)]
struct AssignmentStored {
    id: Uuid,
    #[sqlx(rename = "subject_type")]
    subject_type: SubjectType,
    subject_id: Uuid,
    tenant_id: Option<Uuid>,
    system_id: Uuid,
    visible: bool,
    accessible: bool,
    system_roles: Vec<String>,
    permissions: Vec<String>,
    scopes: Value,
    source_note: Option<String>,
    starts_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow, Serialize)]
struct SystemListRow {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    icon_url: Option<String>,
    entry_url: String,
    callback_url: Option<String>,
    #[sqlx(rename = "status")]
    status: SystemStatus,
    portal_managed: bool,
    auth_enabled: bool,
    supports_sub_admin: bool,
    supported_identity_levels: Value,
    supported_permissions: Value,
    supported_scopes: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    tenant_count: i64,
    assignment_count: i64,
}

#[derive(sqlx::FromRow, Serialize)]
struct SystemDetailRow {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    icon_url: Option<String>,
    entry_url: String,
    callback_url: Option<String>,
    #[sqlx(rename = "status")]
    status: SystemStatus,
    portal_managed: bool,
    auth_enabled: bool,
    supports_sub_admin: bool,
    supported_identity_levels: Value,
    supported_permissions: Value,
    supported_scopes: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    tenant_count: i64,
    assignment_count: i64,
    grant_count: i64,
}

#[derive(sqlx::FromRow, Serialize)]
struct SystemStatusRow {
    id: Uuid,
    code: String,
    name: String,
    #[sqlx(rename = "status")]
    status: SystemStatus,
}

#[derive(sqlx::FromRow)]
struct SystemTenantRow {
    id: Uuid,
    tenant_id: Uuid,
    enabled: bool,
    code: String,
    name: String,
    #[sqlx(rename = "status")]
    status: TenantStatus,
}

#[derive(sqlx::FromRow, Serialize)]
struct IntegrationRow {
    system_id: Uuid,
    issuer: String,
    #[sqlx(rename = "auth_mode")]
    auth_mode: crate::models::AuthMode,
    token_ttl_seconds: i32,
    public_key: Option<String>,
    verify_endpoint: Option<String>,
    env_template: Value,
    last_check_at: Option<DateTime<Utc>>,
    last_check_result: Option<Value>,
}

#[derive(sqlx::FromRow)]
struct IntegrationListRow {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    #[sqlx(rename = "status")]
    status: SystemStatus,
    entry_url: String,
    callback_url: Option<String>,
    auth_enabled: bool,
    supports_sub_admin: bool,
    issuer: Option<String>,
    #[sqlx(rename = "auth_mode")]
    auth_mode: Option<crate::models::AuthMode>,
    token_ttl_seconds: Option<i32>,
    verify_endpoint: Option<String>,
    last_check_at: Option<DateTime<Utc>>,
    last_check_result: Option<Value>,
}

#[derive(sqlx::FromRow)]
struct SystemScopeRow {
    id: Uuid,
    code: String,
    supports_sub_admin: bool,
    supported_scopes: Value,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct SystemCheckRow {
    id: Uuid,
    code: String,
    callback_url: Option<String>,
    #[sqlx(rename = "auth_mode")]
    auth_mode: Option<crate::models::AuthMode>,
}

#[derive(sqlx::FromRow)]
struct GrantListRow {
    id: Uuid,
    user_id: Uuid,
    username: String,
    display_name: String,
    tenant_id: Option<Uuid>,
    tenant_name: Option<String>,
    system_id: Uuid,
    system_code: String,
    system_name: String,
    #[sqlx(rename = "admin_type")]
    admin_type: AdminType,
    scopes: Value,
    #[sqlx(rename = "status")]
    status: GrantStatus,
    reason: Option<String>,
    starts_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct GrantUpdateRow {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    system_id: Uuid,
    #[sqlx(rename = "admin_type")]
    admin_type: AdminType,
    scopes: Value,
    #[sqlx(rename = "status")]
    status: GrantStatus,
    reason: Option<String>,
    starts_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct GrantDetailRow {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    system_id: Uuid,
    #[sqlx(rename = "admin_type")]
    admin_type: AdminType,
    scopes: Value,
    #[sqlx(rename = "status")]
    status: GrantStatus,
    reason: Option<String>,
    starts_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    created_by: Option<Uuid>,
    updated_by: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SubjectRow {
    id: Uuid,
    code: String,
    name: String,
    #[sqlx(default)]
    status: Option<UserStatus>,
}
