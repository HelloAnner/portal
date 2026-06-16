use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::error::AppError;

pub async fn connect_database(config: &AppConfig) -> Result<PgPool, AppError> {
    let db = &config.database;
    let url = db.url();

    // Ensure database exists in development
    if config.app_env == "development" {
        let maintenance_url = format!(
            "postgres://{}:{}@{}:{}/postgres",
            urlencoding::encode(&db.user),
            urlencoding::encode(&db.password),
            db.host,
            db.port
        );
        if let Ok(maintenance_pool) = PgPoolOptions::new()
            .max_connections(1)
            .connect(&maintenance_url)
            .await
        {
            let create_db = format!(
                "SELECT 1 FROM pg_database WHERE datname = '{}'",
                db.database
            );
            let exists: bool = sqlx::query_scalar::<_, bool>(&create_db)
                .fetch_optional(&maintenance_pool)
                .await
                .ok()
                .flatten()
                .is_some();
            if !exists {
                let create_sql = format!(
                    "CREATE DATABASE \"{}\"",
                    db.database.replace('"', "\\\"")
                );
                if let Err(e) = maintenance_pool.execute(&*create_sql).await {
                    warn!("failed to auto-create database: {}", e);
                }
            }
        }
    }

    let schema = &db.schema;
    let search_path = format!(
        "SET search_path TO \"{}\", public",
        schema.replace('"', "\\\"")
    );
    let connection_search_path = search_path.clone();
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .after_connect(move |conn, _meta| {
            let search_path = connection_search_path.clone();
            Box::pin(async move {
                conn.execute(search_path.as_str()).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("FATAL: cannot connect to PostgreSQL: {}", e);
            std::process::exit(1);
        }
    };

    // Ensure schema exists and set search_path
    let schema_quoted = format!("\"{}\"", schema.replace('"', "\\\""));
    pool.execute(format!("CREATE SCHEMA IF NOT EXISTS {}", schema_quoted).as_str())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("create schema failed: {}", e)))?;

    pool.execute(search_path.as_str())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("set search_path failed: {}", e)))?;

    // Test connection
    let _row: (i32,) = match sqlx::query_as("SELECT 1").fetch_one(&pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("FATAL: PostgreSQL health check failed: {}", e);
            std::process::exit(1);
        }
    };
    info!("connected to PostgreSQL, schema={}", schema);

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    let migrations = include_str!("../migrations/0001_init.sql");
    sqlx::raw_sql(migrations)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("migration failed: {}", e)))?;
    Ok(())
}
