use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn user_exists(pool: &PgPool, user_id: Uuid) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM portal_users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("user exists failed: {}", e)))?;
    Ok(exists)
}
