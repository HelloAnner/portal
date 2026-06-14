use std::collections::{HashMap, HashSet};

use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{AdminScope, AdminType, SystemStatus};

#[derive(Debug, Clone)]
pub struct EffectiveSystemContext {
    pub system_id: Uuid,
    pub system_code: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub entry_url: String,
    pub category: Option<String>,
    pub status: SystemStatus,
    pub visible: bool,
    pub accessible: bool,
    pub system_roles: Vec<String>,
    pub permissions: Vec<String>,
    pub admin_scopes: Vec<AdminScope>,
    pub identity_label: String,
    pub is_sub_admin: bool,
    pub admin_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PermissionSource {
    pub source_type: String,
    pub label: String,
    pub visible: bool,
    pub accessible: bool,
    pub system_roles: Vec<String>,
    pub permissions: Vec<String>,
    pub admin_scopes: Vec<AdminScope>,
}

#[derive(Debug, Clone)]
pub struct PermissionDetails {
    pub context: EffectiveSystemContext,
    pub sources: Vec<PermissionSource>,
    pub source_summary: Vec<String>,
    pub scope_summary: Vec<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SystemRow {
    id: Uuid,
    code: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    icon_url: Option<String>,
    entry_url: String,
    status: SystemStatus,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct AssignmentRow {
    system_id: Uuid,
    visible: bool,
    accessible: bool,
    system_roles: Option<Vec<String>>,
    permissions: Option<Vec<String>>,
    scopes: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct GrantRow {
    system_id: Uuid,
    admin_type: AdminType,
    scopes: Value,
}

pub async fn list_effective_contexts(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
) -> Result<Vec<EffectiveSystemContext>, AppError> {
    let ctx = load_and_compute(pool, user_id, tenant_id, None).await?;
    Ok(ctx.contexts.into_iter().filter(|c| c.visible).collect())
}

pub async fn get_effective_context(
    pool: &PgPool,
    user_id: Uuid,
    system_id: Uuid,
    tenant_id: Uuid,
) -> Result<Option<EffectiveSystemContext>, AppError> {
    let ctx = load_and_compute(pool, user_id, tenant_id, Some(system_id)).await?;
    Ok(ctx.contexts.into_iter().next())
}

pub async fn get_permission_details(
    pool: &PgPool,
    user_id: Uuid,
    system_id: Uuid,
    tenant_id: Uuid,
) -> Result<PermissionDetails, AppError> {
    let mut ctx = load_and_compute(pool, user_id, tenant_id, Some(system_id)).await?;
    if ctx.contexts.is_empty() {
        return Ok(PermissionDetails {
            context: empty_context(system_id, "".to_string()),
            sources: vec![],
            source_summary: vec![],
            scope_summary: vec![],
        });
    }
    let context = ctx.contexts.pop().unwrap();
    let sources = ctx.sources.remove(&system_id).unwrap_or_default();
    let source_summary = build_source_summary(&sources);
    let scope_summary = build_scope_summary(&sources);
    Ok(PermissionDetails {
        context,
        sources,
        source_summary,
        scope_summary,
    })
}

struct Computation {
    contexts: Vec<EffectiveSystemContext>,
    sources: HashMap<Uuid, Vec<PermissionSource>>,
}

async fn load_and_compute(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    system_filter: Option<Uuid>,
) -> Result<Computation, AppError> {
    let active = check_membership_active(pool, user_id, tenant_id).await?;
    if !active {
        return Ok(Computation {
            contexts: vec![],
            sources: HashMap::new(),
        });
    }

    let systems = load_systems(pool, system_filter).await?;
    if systems.is_empty() {
        return Ok(Computation {
            contexts: vec![],
            sources: HashMap::new(),
        });
    }

    let system_ids: Vec<Uuid> = systems.iter().map(|s| s.id).collect();

    let (tenant_systems, user_assignments, role_assignments, tenant_assignments, sub_admin_grants) = futures_util::try_join!(
        load_tenant_systems(pool, tenant_id, &system_ids),
        load_user_assignments(pool, user_id, tenant_id, &system_ids),
        load_role_assignments(pool, user_id, tenant_id, &system_ids),
        load_tenant_assignments(pool, tenant_id, &system_ids),
        load_sub_admin_grants(pool, user_id, tenant_id, &system_ids),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("permission load failed: {}", e)))?;

    let mut contexts = Vec::with_capacity(systems.len());
    let mut sources_map: HashMap<Uuid, Vec<PermissionSource>> = HashMap::new();

    for system in systems {
        let enabled = tenant_systems.get(&system.id).copied().unwrap_or(false);
        let user_asgs: Vec<&AssignmentRow> = user_assignments
            .iter()
            .filter(|a| a.system_id == system.id)
            .collect();
        let role_asgs: Vec<&AssignmentRow> = role_assignments
            .iter()
            .filter(|a| a.system_id == system.id)
            .collect();
        let tenant_asgs: Vec<&AssignmentRow> = if enabled {
            tenant_assignments
                .iter()
                .filter(|a| a.system_id == system.id)
                .collect()
        } else {
            vec![]
        };
        let grants: Vec<&GrantRow> = sub_admin_grants
            .iter()
            .filter(|g| g.system_id == system.id)
            .collect();

        let has_explicit_deny = user_asgs.iter().any(|a| !a.visible);

        let mut visible = false;
        let mut accessible = false;
        let mut system_roles: HashSet<String> = HashSet::new();
        let mut permissions: HashSet<String> = HashSet::new();
        let mut admin_scopes: HashMap<String, AdminScope> = HashMap::new();
        let mut is_sub_admin = false;
        let mut admin_type: Option<String> = None;
        let mut sources: Vec<PermissionSource> = Vec::new();

        if !has_explicit_deny {
            for a in &user_asgs {
                if a.visible {
                    visible = true;
                }
                if a.accessible {
                    accessible = true;
                }
                for r in a.system_roles.as_deref().unwrap_or(&[]) {
                    system_roles.insert(r.clone());
                }
                for p in a.permissions.as_deref().unwrap_or(&[]) {
                    permissions.insert(p.clone());
                }
                for s in parse_scopes(&a.scopes) {
                    admin_scopes.insert(format!("{}:{}", s.scope_type, s.scope_code), s);
                }
                sources.push(PermissionSource {
                    source_type: "direct".to_string(),
                    label: "直接授权".to_string(),
                    visible: a.visible,
                    accessible: a.accessible,
                    system_roles: a.system_roles.clone().unwrap_or_default(),
                    permissions: a.permissions.clone().unwrap_or_default(),
                    admin_scopes: parse_scopes(&a.scopes),
                });
            }

            for a in &role_asgs {
                if a.visible {
                    visible = true;
                }
                if a.accessible {
                    accessible = true;
                }
                for r in a.system_roles.as_deref().unwrap_or(&[]) {
                    system_roles.insert(r.clone());
                }
                for p in a.permissions.as_deref().unwrap_or(&[]) {
                    permissions.insert(p.clone());
                }
                for s in parse_scopes(&a.scopes) {
                    admin_scopes.insert(format!("{}:{}", s.scope_type, s.scope_code), s);
                }
                sources.push(PermissionSource {
                    source_type: "role".to_string(),
                    label: "角色授权".to_string(),
                    visible: a.visible,
                    accessible: a.accessible,
                    system_roles: a.system_roles.clone().unwrap_or_default(),
                    permissions: a.permissions.clone().unwrap_or_default(),
                    admin_scopes: parse_scopes(&a.scopes),
                });
            }

            for a in &tenant_asgs {
                if a.visible {
                    visible = true;
                }
                if a.accessible {
                    accessible = true;
                }
                for r in a.system_roles.as_deref().unwrap_or(&[]) {
                    system_roles.insert(r.clone());
                }
                for p in a.permissions.as_deref().unwrap_or(&[]) {
                    permissions.insert(p.clone());
                }
                for s in parse_scopes(&a.scopes) {
                    admin_scopes.insert(format!("{}:{}", s.scope_type, s.scope_code), s);
                }
                sources.push(PermissionSource {
                    source_type: "tenant".to_string(),
                    label: "租户默认".to_string(),
                    visible: a.visible,
                    accessible: a.accessible,
                    system_roles: a.system_roles.clone().unwrap_or_default(),
                    permissions: a.permissions.clone().unwrap_or_default(),
                    admin_scopes: parse_scopes(&a.scopes),
                });
            }

            for g in &grants {
                visible = true;
                accessible = true;
                is_sub_admin = true;
                system_roles.insert("admin".to_string());
                let admin_role = admin_type_to_role(&g.admin_type);
                system_roles.insert(admin_role.clone());
                admin_type = Some(format!("{:?}", g.admin_type).to_lowercase());
                for s in parse_scopes(&g.scopes) {
                    admin_scopes.insert(format!("{}:{}", s.scope_type, s.scope_code), s);
                }
                sources.push(PermissionSource {
                    source_type: "subadmin".to_string(),
                    label: format!("子管理员：{}", admin_type_label(&g.admin_type)),
                    visible: true,
                    accessible: true,
                    system_roles: vec![admin_role],
                    permissions: vec![],
                    admin_scopes: parse_scopes(&g.scopes),
                });
            }
        }

        if !matches!(system.status, SystemStatus::Active) {
            accessible = false;
        }
        if matches!(system.status, SystemStatus::Maintenance) {
            visible = true;
            accessible = false;
        }

        let role_list: Vec<String> = system_roles.into_iter().collect();
        let scope_list: Vec<AdminScope> = admin_scopes.into_values().collect();
        let identity_label = derive_identity_label(&role_list, &scope_list);

        contexts.push(EffectiveSystemContext {
            system_id: system.id,
            system_code: system.code.clone(),
            name: system.name.clone(),
            description: system.description.clone(),
            icon_url: system.icon_url.clone(),
            entry_url: system.entry_url.clone(),
            category: system.category.clone(),
            status: system.status.clone(),
            visible,
            accessible,
            system_roles: role_list,
            permissions: permissions.into_iter().collect(),
            admin_scopes: scope_list,
            identity_label,
            is_sub_admin,
            admin_type,
        });
        sources_map.insert(system.id, sources);
    }

    Ok(Computation {
        contexts,
        sources: sources_map,
    })
}

async fn check_membership_active(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
) -> Result<bool, AppError> {
    let active: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM portal_users u
            JOIN portal_tenant_members tm ON tm.user_id = u.id
            JOIN portal_tenants t ON t.id = tm.tenant_id
            WHERE u.id = $1 AND u.status = 'active'
              AND tm.tenant_id = $2 AND tm.member_status = 'active'
              AND t.status = 'active'
        )"#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("membership check failed: {}", e)))?;
    Ok(active)
}

async fn load_systems(
    pool: &PgPool,
    filter: Option<Uuid>,
) -> Result<Vec<SystemRow>, AppError> {
    let result = if let Some(id) = filter {
        sqlx::query_as::<_, SystemRow>(
            r#"SELECT id, code, name, description, category, icon_url, entry_url, status
               FROM portal_systems
               WHERE id = $1 AND status IN ('active', 'onboarding', 'maintenance')"#,
        )
        .bind(id)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, SystemRow>(
            r#"SELECT id, code, name, description, category, icon_url, entry_url, status
               FROM portal_systems
               WHERE status IN ('active', 'onboarding', 'maintenance')"#,
        )
        .fetch_all(pool)
        .await
    };
    result.map_err(|e| AppError::Internal(anyhow::anyhow!("load systems failed: {}", e)))
}

async fn load_tenant_systems(
    pool: &PgPool,
    tenant_id: Uuid,
    system_ids: &[Uuid],
) -> Result<HashMap<Uuid, bool>, sqlx::Error> {
    if system_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(Uuid, bool)> = sqlx::query_as(
        r#"SELECT system_id, enabled FROM portal_tenant_systems
           WHERE tenant_id = $1 AND system_id = ANY($2)"#,
    )
    .bind(tenant_id)
    .bind(system_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

async fn load_user_assignments(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    system_ids: &[Uuid],
) -> Result<Vec<AssignmentRow>, sqlx::Error> {
    if system_ids.is_empty() {
        return Ok(vec![]);
    }
    sqlx::query_as::<_, AssignmentRow>(
        r#"SELECT system_id, visible, accessible, system_roles, permissions, scopes
           FROM portal_permission_assignments
           WHERE subject_type = 'user' AND subject_id = $1 AND tenant_id = $2 AND system_id = ANY($3)"#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(system_ids)
    .fetch_all(pool)
    .await
}

async fn load_role_assignments(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    system_ids: &[Uuid],
) -> Result<Vec<AssignmentRow>, sqlx::Error> {
    if system_ids.is_empty() {
        return Ok(vec![]);
    }
    let role_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"SELECT role_id FROM portal_user_roles
           WHERE user_id = $1 AND (tenant_id = $2 OR tenant_id IS NULL)"#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    if role_ids.is_empty() {
        return Ok(vec![]);
    }

    sqlx::query_as::<_, AssignmentRow>(
        r#"SELECT system_id, visible, accessible, system_roles, permissions, scopes
           FROM portal_permission_assignments
           WHERE subject_type = 'role' AND subject_id = ANY($1) AND tenant_id = $2 AND system_id = ANY($3)"#,
    )
    .bind(&role_ids)
    .bind(tenant_id)
    .bind(system_ids)
    .fetch_all(pool)
    .await
}

async fn load_tenant_assignments(
    pool: &PgPool,
    tenant_id: Uuid,
    system_ids: &[Uuid],
) -> Result<Vec<AssignmentRow>, sqlx::Error> {
    if system_ids.is_empty() {
        return Ok(vec![]);
    }
    sqlx::query_as::<_, AssignmentRow>(
        r#"SELECT system_id, visible, accessible, system_roles, permissions, scopes
           FROM portal_permission_assignments
           WHERE subject_type = 'tenant' AND subject_id = $1 AND tenant_id = $1 AND system_id = ANY($2)"#,
    )
    .bind(tenant_id)
    .bind(system_ids)
    .fetch_all(pool)
    .await
}

async fn load_sub_admin_grants(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    system_ids: &[Uuid],
) -> Result<Vec<GrantRow>, sqlx::Error> {
    if system_ids.is_empty() {
        return Ok(vec![]);
    }
    sqlx::query_as::<_, GrantRow>(
        r#"SELECT system_id, admin_type, scopes
           FROM portal_sub_admin_grants
           WHERE user_id = $1 AND (tenant_id = $2 OR tenant_id IS NULL)
             AND system_id = ANY($3)
             AND status = 'active'
             AND (expires_at IS NULL OR expires_at > NOW())"#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(system_ids)
    .fetch_all(pool)
    .await
}

fn parse_scopes(value: &Value) -> Vec<AdminScope> {
    serde_json::from_value::<Vec<AdminScope>>(value.clone()).unwrap_or_default()
}

fn admin_type_to_role(admin_type: &AdminType) -> String {
    match admin_type {
        AdminType::System => "super-admin".to_string(),
        AdminType::Tenant => "tenant-admin".to_string(),
        AdminType::Module => "module-admin".to_string(),
        AdminType::Resource | AdminType::Organization => "admin".to_string(),
    }
}

fn admin_type_label(admin_type: &AdminType) -> String {
    match admin_type {
        AdminType::System => "系统级".to_string(),
        AdminType::Tenant => "租户级".to_string(),
        AdminType::Module => "模块级".to_string(),
        AdminType::Resource => "资源级".to_string(),
        AdminType::Organization => "组织级".to_string(),
    }
}

fn derive_identity_label(system_roles: &[String], admin_scopes: &[AdminScope]) -> String {
    if system_roles.contains(&"super-admin".to_string()) {
        "超级管理员".to_string()
    } else if system_roles.contains(&"tenant-admin".to_string()) {
        "租户管理员".to_string()
    } else if system_roles.contains(&"module-admin".to_string()) || !admin_scopes.is_empty() {
        "模块管理员".to_string()
    } else if system_roles.contains(&"admin".to_string()) {
        "管理员".to_string()
    } else {
        "普通用户".to_string()
    }
}

fn build_source_summary(sources: &[PermissionSource]) -> Vec<String> {
    sources
        .iter()
        .filter(|s| s.visible || s.source_type == "subadmin")
        .map(|s| s.label.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn build_scope_summary(sources: &[PermissionSource]) -> Vec<String> {
    let mut seen = HashSet::new();
    for s in sources {
        for scope in &s.admin_scopes {
            let label = format!("{}:{}", scope.scope_type, scope.scope_code);
            seen.insert(label);
        }
    }
    seen.into_iter().collect()
}

fn empty_context(system_id: Uuid, system_code: String) -> EffectiveSystemContext {
    EffectiveSystemContext {
        system_id,
        system_code,
        name: String::new(),
        description: None,
        icon_url: None,
        entry_url: String::new(),
        category: None,
        status: SystemStatus::Disabled,
        visible: false,
        accessible: false,
        system_roles: vec![],
        permissions: vec![],
        admin_scopes: vec![],
        identity_label: String::new(),
        is_sub_admin: false,
        admin_type: None,
    }
}
