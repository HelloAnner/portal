use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn tenant_exists(pool: &PgPool, tenant_id: Uuid) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_tenants WHERE id = $1)")
        .bind(tenant_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("tenant exists failed: {}", e)))?;
    Ok(exists)
}
