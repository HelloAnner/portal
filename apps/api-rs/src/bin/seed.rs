use portal_api::{config::AppConfig, db::{connect_database, run_migrations}};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = AppConfig::from_env();
    let db = connect_database(&config).await?;
    run_migrations(&db).await?;

    let admin_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let password_hash = bcrypt::hash("admin123", bcrypt::DEFAULT_COST)?;

    // Insert default tenant
    sqlx::query(
        r#"INSERT INTO portal_tenants (id, code, name, status, description)
           VALUES ($1, $2, $3, $4::"TenantStatus", $5)
           ON CONFLICT (code) DO NOTHING"#,
    )
    .bind(tenant_id)
    .bind("default")
    .bind("默认租户")
    .bind("active")
    .bind("系统默认租户")
    .execute(&db)
    .await?;

    // Insert builtin roles
    let roles = vec![
        (Uuid::new_v4(), "super-admin", "超级管理员", "super_admin"),
        (Uuid::new_v4(), "user", "普通用户", "normal"),
        (Uuid::new_v4(), "subsystem-admin", "子系统管理员", "subsystem_admin"),
        (Uuid::new_v4(), "audit-viewer", "审计查看员", "custom"),
        (Uuid::new_v4(), "system-integrator", "系统集成员", "custom"),
    ];
    for (id, code, name, role_type) in roles {
        sqlx::query(
            r#"INSERT INTO portal_roles (id, code, name, role_type, is_builtin)
               VALUES ($1, $2, $3, $4::"RoleType", true)
               ON CONFLICT (code) DO NOTHING"#,
        )
        .bind(id)
        .bind(code)
        .bind(name)
        .bind(role_type)
        .execute(&db)
        .await?;
    }

    // Insert admin user
    sqlx::query(
        r#"INSERT INTO portal_users (id, username, password_hash, display_name, email, status, default_tenant_id)
           VALUES ($1, $2, $3, $4, $5, $6::"UserStatus", $7)
           ON CONFLICT (username) DO NOTHING"#,
    )
    .bind(admin_id)
    .bind("admin")
    .bind(password_hash)
    .bind("系统管理员")
    .bind("admin@example.com")
    .bind("active")
    .bind(tenant_id)
    .execute(&db)
    .await?;

    // Assign super-admin role
    let super_admin_id: Uuid = sqlx::query_scalar("SELECT id FROM portal_roles WHERE code = $1")
        .bind("super-admin")
        .fetch_one(&db)
        .await?;

    sqlx::query(
        r#"INSERT INTO portal_user_roles (id, user_id, role_id, tenant_id)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id, role_id, tenant_id) DO NOTHING"#,
    )
    .bind(Uuid::new_v4())
    .bind(admin_id)
    .bind(super_admin_id)
    .bind(tenant_id)
    .execute(&db)
    .await?;

    // Add admin as tenant member
    sqlx::query(
        r#"INSERT INTO portal_tenant_members (id, tenant_id, user_id)
           VALUES ($1, $2, $3)
           ON CONFLICT (tenant_id, user_id) DO NOTHING"#,
    )
    .bind(Uuid::new_v4())
    .bind(tenant_id)
    .bind(admin_id)
    .execute(&db)
    .await?;

    // Insert default systems
    let systems = vec![
        (
            Uuid::new_v4(),
            "northline",
            "Northline",
            "企业数据库问数系统",
            "http://localhost:3001",
            "http://localhost:3001/auth/callback",
        ),
        (
            Uuid::new_v4(),
            "documind",
            "DocuMind",
            "企业文档 RAG 系统",
            "http://localhost:3002",
            "http://localhost:3002/auth/callback",
        ),
    ];
    for (id, code, name, desc, entry, callback) in systems {
        sqlx::query(
            r#"INSERT INTO portal_systems (id, code, name, description, entry_url, callback_url, status, supports_sub_admin)
               VALUES ($1, $2, $3, $4, $5, $6, $7::"SystemStatus", true)
               ON CONFLICT (code) DO NOTHING"#,
        )
        .bind(id)
        .bind(code)
        .bind(name)
        .bind(desc)
        .bind(entry)
        .bind(callback)
        .bind("active")
        .execute(&db)
        .await?;

        // Enable for default tenant
        sqlx::query(
            r#"INSERT INTO portal_tenant_systems (id, tenant_id, system_id)
               VALUES ($1, $2, $3)
               ON CONFLICT (tenant_id, system_id) DO NOTHING"#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(id)
        .execute(&db)
        .await?;

        // Integration config
        sqlx::query(
            r#"INSERT INTO portal_integration_configs (system_id, issuer, auth_mode)
               VALUES ($1, $2, $3::"AuthMode")
               ON CONFLICT (system_id) DO NOTHING"#,
        )
        .bind(id)
        .bind(&config.jwt.issuer)
        .bind("authorization_code")
        .execute(&db)
        .await?;

        // Grant super-admin role full access to all systems
        sqlx::query(
            r#"INSERT INTO portal_permission_assignments
               (id, subject_type, subject_id, tenant_id, system_id, visible, accessible, system_roles, permissions)
               VALUES ($1, $2::"SubjectType", $3, $4, $5, true, true, $6, $7)"#,
        )
        .bind(Uuid::new_v4())
        .bind("role")
        .bind(super_admin_id)
        .bind(tenant_id)
        .bind(id)
        .bind(&["admin"][..])
        .bind(&["system:*"][..])
        .execute(&db)
        .await?;
    }

    println!("seed completed: admin/admin123, tenant=default");
    Ok(())
}
