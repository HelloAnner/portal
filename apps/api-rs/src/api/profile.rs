use std::net::SocketAddr;
use std::path::Path;

use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap},
    routing::{get, post},
    Extension, Json, Router,
};
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::CurrentSession;
use crate::error::AppError;
use crate::middleware::RequestId;
use crate::models::UserStatus;
use crate::services::audit::{write_audit, AuditPayload};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/profile", get(get_profile).patch(update_profile))
        .route("/api/profile/avatar", post(upload_avatar))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileResponse {
    username: String,
    display_name: String,
    email: Option<String>,
    phone: Option<String>,
    organization_path: Option<String>,
    status: UserStatus,
    avatar_url: Option<String>,
    default_tenant_id: Option<Uuid>,
    preferences: Value,
}

async fn get_profile(session: CurrentSession) -> Result<Json<ProfileResponse>, AppError> {
    Ok(Json(ProfileResponse {
        username: session.user.username,
        display_name: session.user.display_name,
        email: session.user.email,
        phone: session.user.phone,
        organization_path: session.user.organization_path,
        status: session.user.status,
        avatar_url: session.user.avatar_url,
        default_tenant_id: session.user.default_tenant_id,
        preferences: session.user.preferences,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProfileRequest {
    display_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    organization_path: Option<String>,
    preferences: Option<Value>,
    avatar_url: Option<String>,
    default_tenant_id: Option<Uuid>,
}

async fn update_profile(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    session: CurrentSession,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, AppError> {
    if let Some(ref name) = req.display_name {
        if name.trim().is_empty() {
            return Err(AppError::ValidationFailed("display_name cannot be empty".to_string()));
        }
    }

    if let Some(tid) = req.default_tenant_id {
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM portal_tenant_members WHERE tenant_id = $1 AND user_id = $2 AND member_status = 'active')"
        )
        .bind(tid)
        .bind(session.user.id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("membership check failed: {}", e)))?;
        if !active {
            return Err(AppError::ValidationFailed("invalid default tenant".to_string()));
        }
    }

    let before = serde_json::json!({
        "displayName": session.user.display_name,
        "avatarUrl": session.user.avatar_url,
        "defaultTenantId": session.user.default_tenant_id,
        "preferences": session.user.preferences,
    });

    let updated = sqlx::query(
        r#"UPDATE portal_users SET
            display_name = COALESCE($1, display_name),
            email = COALESCE($2, email),
            phone = COALESCE($3, phone),
            organization_path = COALESCE($4, organization_path),
            preferences = COALESCE($5, preferences),
            avatar_url = COALESCE($6, avatar_url),
            default_tenant_id = COALESCE($7, default_tenant_id),
            updated_at = NOW()
           WHERE id = $8
           RETURNING username, display_name, email, phone, organization_path, status as "status",
                     avatar_url, default_tenant_id, preferences"#,
    )
    .bind(req.display_name)
    .bind(req.email)
    .bind(req.phone)
    .bind(req.organization_path)
    .bind(req.preferences)
    .bind(req.avatar_url)
    .bind(req.default_tenant_id)
    .bind(session.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("profile update failed: {}", e)))?;

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    write_audit(
        &state.db,
        AuditPayload {
            request_id: Some(request_id.0),
            actor_user_id: Some(session.user.id),
            action: "profile.update".to_string(),
            target_type: "user".to_string(),
            target_id: Some(session.user.id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: Some(before),
            after_data: Some(serde_json::json!({
                "displayName": updated.get::<String, _>("display_name"),
                "avatarUrl": updated.get::<Option<String>, _>("avatar_url"),
                "defaultTenantId": updated.get::<Option<Uuid>, _>("default_tenant_id"),
                "preferences": updated.get::<Value, _>("preferences"),
            })),
            failure_reason: None,
            ip_address: Some(ip),
            user_agent,
        },
    )
    .await
    .ok();

    Ok(Json(ProfileResponse {
        username: updated.get("username"),
        display_name: updated.get("display_name"),
        email: updated.get("email"),
        phone: updated.get("phone"),
        organization_path: updated.get("organization_path"),
        status: updated.get("status"),
        avatar_url: updated.get("avatar_url"),
        default_tenant_id: updated.get("default_tenant_id"),
        preferences: updated.get("preferences"),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadAvatarRequest {
    base64: Option<String>,
    url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AvatarResponse {
    avatar_url: String,
}

async fn upload_avatar(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    session: CurrentSession,
    Json(req): Json<UploadAvatarRequest>,
) -> Result<Json<AvatarResponse>, AppError> {
    let buffer = match (&req.base64, &req.url) {
        (Some(b), None) => decode_base64_image(b)
            .map_err(|_| AppError::ValidationFailed("INVALID_BASE64".to_string()))?,
        (None, Some(u)) => fetch_image(u)
            .await
            .map_err(|_| AppError::ValidationFailed("FETCH_AVATAR_FAILED".to_string()))?,
        _ => return Err(AppError::ValidationFailed("provide either base64 or url".to_string())),
    };

    if buffer.is_empty() {
        return Err(AppError::ValidationFailed("EMPTY_AVATAR".to_string()));
    }

    let avatars_dir = Path::new("apps/web/public/avatars");
    tokio::fs::create_dir_all(avatars_dir)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("create avatars dir failed: {}", e)))?;

    let file_name = format!("{}.png", session.user.id);
    let file_path = avatars_dir.join(&file_name);
    tokio::fs::write(&file_path, buffer)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("write avatar failed: {}", e)))?;

    let timestamp = Utc::now().timestamp_millis();
    let avatar_url = format!("/avatars/{}?t={}", file_name, timestamp);

    let before = serde_json::json!({
        "avatarUrl": session.user.avatar_url,
    });

    let updated = sqlx::query(
        "UPDATE portal_users SET avatar_url = $1, updated_at = NOW() WHERE id = $2 RETURNING avatar_url"
    )
    .bind(&avatar_url)
    .bind(session.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("avatar update failed: {}", e)))?;

    let ip = addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    write_audit(
        &state.db,
        AuditPayload {
            request_id: Some(request_id.0),
            actor_user_id: Some(session.user.id),
            action: "profile.avatar.update".to_string(),
            target_type: "user".to_string(),
            target_id: Some(session.user.id.to_string()),
            system_id: None,
            tenant_id: None,
            result: "success",
            before_data: Some(before),
            after_data: Some(serde_json::json!({
                "avatarUrl": updated.get::<String, _>("avatar_url"),
            })),
            failure_reason: None,
            ip_address: Some(ip),
            user_agent,
        },
    )
    .await
    .ok();

    Ok(Json(AvatarResponse { avatar_url }))
}

fn decode_base64_image(base64: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let normalized: String = base64.chars().filter(|c| !c.is_whitespace()).collect();
    let data = if let Some(idx) = normalized.find(',') {
        &normalized[idx + 1..]
    } else {
        &normalized
    };
    Ok(base64::engine::general_purpose::STANDARD.decode(data)?)
}

async fn fetch_image(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err("fetch failed".into());
    }
    Ok(resp.bytes().await?.to_vec())
}
