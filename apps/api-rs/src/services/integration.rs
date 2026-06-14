use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn integration_exists(pool: &PgPool, system_id: Uuid) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_integration_configs WHERE system_id = $1)")
        .bind(system_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("integration exists failed: {}", e)))?;
    Ok(exists)
}
