use bcrypt::{hash, DEFAULT_COST};
use chrono::{Duration, Utc};
use rand::Rng;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    hash(password, DEFAULT_COST).map_err(|e| AppError::Internal(anyhow::anyhow!("hash error: {}", e)))
}

pub fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..48).map(|_| rng.gen::<u8>()).collect();
    hex::encode(bytes)
}

pub async fn create_session(
    pool: &Pool<Postgres>,
    user_id: Uuid,
    remember_me: bool,
    ttl_seconds: i64,
    remember_ttl_seconds: i64,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Result<(Uuid, String), AppError> {
    let secret = generate_session_token();
    let hash = hash(&secret, 10).map_err(|e| AppError::Internal(anyhow::anyhow!("session hash error: {}", e)))?;
    let expires_at = Utc::now()
        + Duration::seconds(if remember_me { remember_ttl_seconds } else { ttl_seconds });
    let session_id = Uuid::new_v4();

    sqlx::query(
        r#"INSERT INTO portal_sessions (id, user_id, session_hash, remember_me, expires_at, ip_address, user_agent)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(session_id)
    .bind(user_id)
    .bind(hash)
    .bind(remember_me)
    .bind(expires_at)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("create session failed: {}", e)))?;

    Ok((session_id, secret))
}

pub async fn revoke_session(pool: &Pool<Postgres>, session_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE portal_sessions SET revoked_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("revoke session failed: {}", e)))?;
    Ok(())
}

pub async fn revoke_all_user_sessions(pool: &Pool<Postgres>, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE portal_sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("revoke sessions failed: {}", e)))?;
    Ok(())
}
