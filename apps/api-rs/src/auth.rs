use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use bcrypt::verify;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::error::AppError;
use crate::models::{CurrentUser, RoleInfo, TenantInfo, User, UserStatus};
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub user_id: uuid::Uuid,
    pub remember_me: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentSession {
    pub session_id: uuid::Uuid,
    pub user: User,
}

#[async_trait::async_trait]
impl FromRequestParts<AppState> for CurrentSession {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let cookie_header = parts
            .headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::AuthRequired)?;

        let session_cookie_name = &state.config.session.cookie_name;
        let token = cookie_header
            .split(';')
            .map(|s| s.trim())
            .find_map(|s| s.strip_prefix(&format!("{}=", session_cookie_name)))
            .ok_or(AppError::AuthRequired)?;

        // token format: session_id:secret
        let (session_id_str, secret) = token.split_once(':').ok_or(AppError::AuthRequired)?;
        let session_id = session_id_str.parse::<uuid::Uuid>().map_err(|_| AppError::AuthRequired)?;

        let row = sqlx::query(
            r#"SELECT s.id, s.session_hash, s.remember_me, s.expires_at, s.revoked_at,
                      u.id as user_id, u.username, u.password_hash, u.display_name, u.email,
                      u.phone, u.avatar_url, u.organization_path, u.status as "status",
                      u.default_tenant_id, u.preferences, u.last_login_at, u.created_at, u.updated_at
               FROM portal_sessions s
               JOIN portal_users u ON u.id = s.user_id
               WHERE s.id = $1"#,
        )
        .bind(session_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("session query failed: {}", e)))?;

        let row = row.ok_or(AppError::AuthRequired)?;
        let hash: String = row.get("session_hash");
        let expires_at: chrono::DateTime<Utc> = row.get("expires_at");
        let revoked_at: Option<chrono::DateTime<Utc>> = row.get("revoked_at");

        if revoked_at.is_some() || expires_at < Utc::now() {
            return Err(AppError::AuthRequired);
        }

        let valid = verify(secret, &hash).map_err(|_| AppError::Internal(anyhow::anyhow!("bcrypt verify error")))?;
        if !valid {
            return Err(AppError::AuthRequired);
        }

        let user = User {
            id: row.get("user_id"),
            username: row.get("username"),
            password_hash: row.get("password_hash"),
            display_name: row.get("display_name"),
            email: row.get("email"),
            phone: row.get("phone"),
            avatar_url: row.get("avatar_url"),
            organization_path: row.get("organization_path"),
            status: row.get("status"),
            default_tenant_id: row.get("default_tenant_id"),
            preferences: row.get("preferences"),
            last_login_at: row.get("last_login_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        if !matches!(user.status, UserStatus::Active) {
            return Err(AppError::AuthRequired);
        }

        parts.extensions.insert(user.id.to_string());

        Ok(CurrentSession { session_id: row.get("id"), user })
    }
}

pub async fn load_current_user(state: &AppState, user_id: uuid::Uuid) -> Result<CurrentUser, AppError> {
    let user_row = sqlx::query(
        r#"SELECT id, username, display_name, email, avatar_url, status as "status", default_tenant_id
           FROM portal_users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load user failed: {}", e)))?;

    let roles: Vec<RoleInfo> = sqlx::query_as(
        r#"SELECT r.id, r.code, r.name, ur.tenant_id
           FROM portal_user_roles ur
           JOIN portal_roles r ON r.id = ur.role_id
           WHERE ur.user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load roles failed: {}", e)))?;

    let tenants: Vec<TenantInfo> = sqlx::query_as(
        r#"SELECT t.id, t.code, t.name
           FROM portal_tenant_members tm
           JOIN portal_tenants t ON t.id = tm.tenant_id
           WHERE tm.user_id = $1 AND tm.member_status = 'active' AND t.status = 'active'
           ORDER BY tm.joined_at ASC"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("load tenants failed: {}", e)))?;

    let is_super_admin = roles.iter().any(|r| r.code == "super-admin");

    Ok(CurrentUser {
        id: user_row.get("id"),
        username: user_row.get("username"),
        display_name: user_row.get("display_name"),
        email: user_row.get("email"),
        avatar_url: user_row.get("avatar_url"),
        status: user_row.get("status"),
        default_tenant_id: user_row.get("default_tenant_id"),
        is_super_admin,
        can_enter_admin: is_super_admin,
        roles,
        tenants,
    })
}

pub fn require_super_admin(current: &CurrentUser) -> Result<(), AppError> {
    if current.is_super_admin {
        Ok(())
    } else {
        Err(AppError::PermissionDenied)
    }
}
