use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub struct AuditPayload {
    pub request_id: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub system_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub result: &'static str,
    pub before_data: Option<Value>,
    pub after_data: Option<Value>,
    pub failure_reason: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

pub async fn write_audit(pool: &PgPool, payload: AuditPayload) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO portal_audit_events
           (id, request_id, actor_user_id, action, target_type, target_id, system_id, tenant_id, result,
            before_data, after_data, failure_reason, ip_address, user_agent)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::AuditResult, $10, $11, $12, $13, $14)"#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.request_id)
    .bind(payload.actor_user_id)
    .bind(payload.action)
    .bind(payload.target_type)
    .bind(payload.target_id)
    .bind(payload.system_id)
    .bind(payload.tenant_id)
    .bind(payload.result)
    .bind(payload.before_data)
    .bind(payload.after_data)
    .bind(payload.failure_reason)
    .bind(payload.ip_address)
    .bind(payload.user_agent)
    .execute(pool)
    .await?;
    Ok(())
}
