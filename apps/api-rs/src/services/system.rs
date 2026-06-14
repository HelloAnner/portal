use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn system_exists(pool: &PgPool, system_id: Uuid) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_systems WHERE id = $1)")
        .bind(system_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("system exists failed: {}", e)))?;
    Ok(exists)
}
