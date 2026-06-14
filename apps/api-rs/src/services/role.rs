use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn role_exists(pool: &PgPool, role_id: Uuid) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_roles WHERE id = $1)")
        .bind(role_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("role exists failed: {}", e)))?;
    Ok(exists)
}
