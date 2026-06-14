use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "UserStatus", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Disabled,
    Pending,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "RoleType", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RoleType {
    SuperAdmin,
    Normal,
    SubsystemAdmin,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TenantStatus", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    Active,
    Disabled,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "SystemStatus", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SystemStatus {
    Active,
    Disabled,
    Onboarding,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "SubjectType", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SubjectType {
    User,
    Role,
    Tenant,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "AdminType", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AdminType {
    System,
    Tenant,
    Module,
    Resource,
    Organization,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "GrantStatus", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GrantStatus {
    Active,
    Inactive,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "AuthMode", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Jwt,
    AuthorizationCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "AuditResult", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: Option<String>,
    pub display_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub avatar_url: Option<String>,
    pub organization_path: Option<String>,
    pub status: UserStatus,
    pub default_tenant_id: Option<Uuid>,
    pub preferences: Value,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub role_type: RoleType,
    pub description: Option<String>,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub status: TenantStatus,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub icon_url: Option<String>,
    pub entry_url: String,
    pub callback_url: Option<String>,
    pub status: SystemStatus,
    pub portal_managed: bool,
    pub auth_enabled: bool,
    pub supports_sub_admin: bool,
    pub supported_identity_levels: Value,
    pub supported_permissions: Value,
    pub supported_scopes: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub status: UserStatus,
    pub default_tenant_id: Option<Uuid>,
    pub is_super_admin: bool,
    pub can_enter_admin: bool,
    pub roles: Vec<RoleInfo>,
    pub tenants: Vec<TenantInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoleInfo {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub tenant_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantInfo {
    pub id: Uuid,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAccess {
    pub system_id: Uuid,
    pub system_code: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub entry_url: String,
    pub category: Option<String>,
    pub status: SystemStatus,
    pub visible: bool,
    pub accessible: bool,
    pub system_roles: Vec<String>,
    pub permissions: Vec<String>,
    pub scopes: Vec<AdminScope>,
    pub identity_label: String,
    pub is_sub_admin: bool,
    pub admin_type: Option<AdminType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminScope {
    pub scope_type: String,
    pub scope_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveContext {
    pub user_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub portal_roles: Vec<String>,
    pub system_access: Vec<SystemAccess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemContext {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub system_code: String,
    pub portal_roles: Vec<String>,
    pub system_roles: Vec<String>,
    pub admin_scopes: Vec<AdminScope>,
    pub permissions: Vec<String>,
    pub issued_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAssignment {
    pub id: Uuid,
    pub subject_type: SubjectType,
    pub subject_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub system_id: Uuid,
    pub visible: bool,
    pub accessible: bool,
    pub system_roles: Vec<String>,
    pub permissions: Vec<String>,
    pub scopes: Value,
    pub source_note: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAdminGrant {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub system_id: Uuid,
    pub admin_type: AdminType,
    pub scopes: Value,
    pub status: GrantStatus,
    pub reason: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub request_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub system_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub result: AuditResult,
    pub before_data: Option<Value>,
    pub after_data: Option<Value>,
    pub failure_reason: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
