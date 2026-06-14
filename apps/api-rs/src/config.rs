use std::env;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub schema: String,
    pub ssl: bool,
}

impl DatabaseConfig {
    pub fn url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            urlencoding::encode(&self.user),
            urlencoding::encode(&self.password),
            self.host,
            self.port,
            self.database
        )
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub cookie_name: String,
    pub ttl_seconds: i64,
    pub remember_me_ttl_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub issuer: String,
    pub token_ttl_seconds: i64,
    pub private_key_path: String,
    pub public_key_path: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub app_port: u16,
    pub app_base_url: String,
    pub database: DatabaseConfig,
    pub session: SessionConfig,
    pub jwt: JwtConfig,
    pub audit_retention_days: i32,
    pub allow_permission_request: bool,
}

fn var(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn var_u16(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn var_i64(key: &str, default: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn var_i32(key: &str, default: i32) -> i32 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn var_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}

impl AppConfig {
    pub fn from_env() -> Self {
        AppConfig {
            app_env: var("APP_ENV", "development"),
            app_port: var_u16("APP_PORT", 8080),
            app_base_url: var("APP_BASE_URL", "http://localhost:8080"),
            database: DatabaseConfig {
                host: var("PG_HOST", "127.0.0.1"),
                port: var_u16("PG_PORT", 5432),
                database: var("PG_DATABASE", "portal"),
                user: var("PG_USER", "portal"),
                password: var("PG_PASSWORD", "portal"),
                schema: var("PG_SCHEMA", "portal"),
                ssl: var_bool("PG_SSL", false),
            },
            session: SessionConfig {
                cookie_name: var("SESSION_COOKIE_NAME", "portal_session"),
                ttl_seconds: var_i64("SESSION_TTL_SECONDS", 28800),
                remember_me_ttl_seconds: var_i64("REMEMBER_ME_TTL_SECONDS", 2592000),
            },
            jwt: JwtConfig {
                issuer: var("PORTAL_ISSUER", "http://localhost:8080"),
                token_ttl_seconds: var_i64("PORTAL_TOKEN_TTL_SECONDS", 300),
                private_key_path: var(
                    "PORTAL_JWT_PRIVATE_KEY_PATH",
                    "./config/portal-private-key.pem",
                ),
                public_key_path: var(
                    "PORTAL_JWT_PUBLIC_KEY_PATH",
                    "./config/portal-public-key.pem",
                ),
            },
            audit_retention_days: var_i32("AUDIT_RETENTION_DAYS", 365),
            allow_permission_request: var_bool("ALLOW_PERMISSION_REQUEST", true),
        }
    }
}
