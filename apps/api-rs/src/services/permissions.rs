use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::SystemAccess;
use crate::permissions::calculator;

pub async fn list_user_system_access(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
) -> Result<Vec<SystemAccess>, AppError> {
    let contexts = calculator::list_effective_contexts(pool, user_id, tenant_id).await?;
    Ok(contexts.into_iter().map(into_system_access).collect())
}

pub async fn get_user_system_access(
    pool: &PgPool,
    user_id: Uuid,
    system_id: Uuid,
    tenant_id: Uuid,
) -> Result<Option<SystemAccess>, AppError> {
    let ctx = calculator::get_effective_context(pool, user_id, system_id, tenant_id).await?;
    Ok(ctx.map(into_system_access))
}

pub async fn get_permission_details(
    pool: &PgPool,
    user_id: Uuid,
    system_id: Uuid,
    tenant_id: Uuid,
) -> Result<calculator::PermissionDetails, AppError> {
    calculator::get_permission_details(pool, user_id, system_id, tenant_id).await
}

fn into_system_access(ctx: calculator::EffectiveSystemContext) -> SystemAccess {
    let admin_type = ctx.admin_type.and_then(|s| match s.as_str() {
        "system" => Some(crate::models::AdminType::System),
        "tenant" => Some(crate::models::AdminType::Tenant),
        "module" => Some(crate::models::AdminType::Module),
        "resource" => Some(crate::models::AdminType::Resource),
        "organization" => Some(crate::models::AdminType::Organization),
        _ => None,
    });
    SystemAccess {
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
        admin_type,
    }
}
