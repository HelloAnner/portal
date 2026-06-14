use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn grant_exists(pool: &PgPool, grant_id: Uuid) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_sub_admin_grants WHERE id = $1)")
        .bind(grant_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("grant exists failed: {}", e)))?;
    Ok(exists)
}
