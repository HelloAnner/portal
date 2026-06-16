DO $$
BEGIN
    IF to_regtype('"UserStatus"') IS NULL THEN
        CREATE TYPE "UserStatus" AS ENUM ('active', 'disabled', 'pending', 'archived');
    END IF;
    IF to_regtype('"RoleType"') IS NULL THEN
        CREATE TYPE "RoleType" AS ENUM ('super_admin', 'normal', 'subsystem_admin', 'custom');
    END IF;
    IF to_regtype('"TenantStatus"') IS NULL THEN
        CREATE TYPE "TenantStatus" AS ENUM ('active', 'disabled', 'archived');
    END IF;
    IF to_regtype('"SystemStatus"') IS NULL THEN
        CREATE TYPE "SystemStatus" AS ENUM ('active', 'disabled', 'onboarding', 'maintenance');
    END IF;
    IF to_regtype('"SubjectType"') IS NULL THEN
        CREATE TYPE "SubjectType" AS ENUM ('user', 'role', 'tenant');
    END IF;
    IF to_regtype('"AdminType"') IS NULL THEN
        CREATE TYPE "AdminType" AS ENUM ('system', 'tenant', 'module', 'resource', 'organization');
    END IF;
    IF to_regtype('"GrantStatus"') IS NULL THEN
        CREATE TYPE "GrantStatus" AS ENUM ('active', 'inactive', 'expired');
    END IF;
    IF to_regtype('"AuthMode"') IS NULL THEN
        CREATE TYPE "AuthMode" AS ENUM ('jwt', 'authorization_code');
    END IF;
    IF to_regtype('"AuditResult"') IS NULL THEN
        CREATE TYPE "AuditResult" AS ENUM ('success', 'failure');
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS "portal_users" (
    "id" UUID NOT NULL PRIMARY KEY,
    "username" TEXT NOT NULL UNIQUE,
    "password_hash" TEXT,
    "display_name" TEXT NOT NULL,
    "email" TEXT,
    "phone" TEXT,
    "avatar_url" TEXT,
    "organization_path" TEXT,
    "status" "UserStatus" NOT NULL,
    "default_tenant_id" UUID,
    "preferences" JSONB NOT NULL DEFAULT '{}',
    "last_login_at" TIMESTAMPTZ(6),
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "portal_roles" (
    "id" UUID NOT NULL PRIMARY KEY,
    "code" TEXT NOT NULL UNIQUE,
    "name" TEXT NOT NULL,
    "role_type" "RoleType" NOT NULL,
    "description" TEXT,
    "is_builtin" BOOLEAN NOT NULL DEFAULT false,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "portal_tenants" (
    "id" UUID NOT NULL PRIMARY KEY,
    "code" TEXT NOT NULL UNIQUE,
    "name" TEXT NOT NULL,
    "status" "TenantStatus" NOT NULL,
    "description" TEXT,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "portal_user_roles" (
    "id" UUID NOT NULL PRIMARY KEY,
    "user_id" UUID NOT NULL REFERENCES "portal_users"("id") ON DELETE CASCADE,
    "role_id" UUID NOT NULL REFERENCES "portal_roles"("id") ON DELETE CASCADE,
    "tenant_id" UUID REFERENCES "portal_tenants"("id") ON DELETE CASCADE,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE ("user_id", "role_id", "tenant_id")
);

CREATE TABLE IF NOT EXISTS "portal_tenant_members" (
    "id" UUID NOT NULL PRIMARY KEY,
    "tenant_id" UUID NOT NULL REFERENCES "portal_tenants"("id") ON DELETE CASCADE,
    "user_id" UUID NOT NULL REFERENCES "portal_users"("id") ON DELETE CASCADE,
    "member_status" TEXT NOT NULL DEFAULT 'active',
    "joined_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE ("tenant_id", "user_id")
);

CREATE TABLE IF NOT EXISTS "portal_systems" (
    "id" UUID NOT NULL PRIMARY KEY,
    "code" TEXT NOT NULL UNIQUE,
    "name" TEXT NOT NULL,
    "description" TEXT,
    "category" TEXT,
    "icon_url" TEXT,
    "entry_url" TEXT NOT NULL,
    "callback_url" TEXT,
    "status" "SystemStatus" NOT NULL,
    "portal_managed" BOOLEAN NOT NULL DEFAULT true,
    "auth_enabled" BOOLEAN NOT NULL DEFAULT true,
    "supports_sub_admin" BOOLEAN NOT NULL DEFAULT false,
    "supported_identity_levels" JSONB NOT NULL DEFAULT '[]',
    "supported_permissions" JSONB NOT NULL DEFAULT '[]',
    "supported_scopes" JSONB NOT NULL DEFAULT '[]',
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "portal_tenant_systems" (
    "id" UUID NOT NULL PRIMARY KEY,
    "tenant_id" UUID NOT NULL REFERENCES "portal_tenants"("id") ON DELETE CASCADE,
    "system_id" UUID NOT NULL REFERENCES "portal_systems"("id") ON DELETE CASCADE,
    "enabled" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE ("tenant_id", "system_id")
);

CREATE TABLE IF NOT EXISTS "portal_permission_assignments" (
    "id" UUID NOT NULL PRIMARY KEY,
    "subject_type" "SubjectType" NOT NULL,
    "subject_id" UUID NOT NULL,
    "tenant_id" UUID REFERENCES "portal_tenants"("id") ON DELETE CASCADE,
    "system_id" UUID NOT NULL REFERENCES "portal_systems"("id") ON DELETE CASCADE,
    "visible" BOOLEAN NOT NULL DEFAULT false,
    "accessible" BOOLEAN NOT NULL DEFAULT false,
    "system_roles" TEXT[] DEFAULT ARRAY[]::TEXT[],
    "permissions" TEXT[] DEFAULT ARRAY[]::TEXT[],
    "scopes" JSONB NOT NULL DEFAULT '[]',
    "source_note" TEXT,
    "starts_at" TIMESTAMPTZ(6),
    "expires_at" TIMESTAMPTZ(6),
    "created_by" UUID REFERENCES "portal_users"("id") ON DELETE SET NULL,
    "updated_by" UUID REFERENCES "portal_users"("id") ON DELETE SET NULL,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "portal_permission_assignments_subject_idx" ON "portal_permission_assignments"("subject_type", "subject_id", "tenant_id", "system_id");

CREATE TABLE IF NOT EXISTS "portal_sub_admin_grants" (
    "id" UUID NOT NULL PRIMARY KEY,
    "user_id" UUID NOT NULL REFERENCES "portal_users"("id") ON DELETE CASCADE,
    "tenant_id" UUID REFERENCES "portal_tenants"("id") ON DELETE CASCADE,
    "system_id" UUID NOT NULL REFERENCES "portal_systems"("id") ON DELETE CASCADE,
    "admin_type" "AdminType" NOT NULL,
    "scopes" JSONB NOT NULL DEFAULT '[]',
    "status" "GrantStatus" NOT NULL,
    "reason" TEXT,
    "starts_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "expires_at" TIMESTAMPTZ(6),
    "created_by" UUID REFERENCES "portal_users"("id") ON DELETE SET NULL,
    "updated_by" UUID REFERENCES "portal_users"("id") ON DELETE SET NULL,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "portal_sub_admin_grants_scope_idx" ON "portal_sub_admin_grants"("user_id", "tenant_id", "system_id", "status");

CREATE TABLE IF NOT EXISTS "portal_integration_configs" (
    "system_id" UUID NOT NULL PRIMARY KEY REFERENCES "portal_systems"("id") ON DELETE CASCADE,
    "issuer" TEXT NOT NULL,
    "auth_mode" "AuthMode" NOT NULL,
    "token_ttl_seconds" INTEGER NOT NULL DEFAULT 300,
    "public_key" TEXT,
    "verify_endpoint" TEXT,
    "env_template" JSONB NOT NULL DEFAULT '{}',
    "last_check_at" TIMESTAMPTZ(6),
    "last_check_result" JSONB,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "portal_sessions" (
    "id" UUID NOT NULL PRIMARY KEY,
    "user_id" UUID NOT NULL REFERENCES "portal_users"("id") ON DELETE CASCADE,
    "session_hash" TEXT NOT NULL UNIQUE,
    "remember_me" BOOLEAN NOT NULL DEFAULT false,
    "expires_at" TIMESTAMPTZ(6) NOT NULL,
    "revoked_at" TIMESTAMPTZ(6),
    "ip_address" TEXT,
    "user_agent" TEXT,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "portal_sessions_user_expires_idx" ON "portal_sessions"("user_id", "expires_at");

CREATE TABLE IF NOT EXISTS "portal_subsystem_tickets" (
    "id" UUID NOT NULL PRIMARY KEY,
    "code_hash" TEXT NOT NULL UNIQUE,
    "user_id" UUID NOT NULL REFERENCES "portal_users"("id") ON DELETE CASCADE,
    "tenant_id" UUID REFERENCES "portal_tenants"("id") ON DELETE CASCADE,
    "system_id" UUID NOT NULL REFERENCES "portal_systems"("id") ON DELETE CASCADE,
    "context_snapshot" JSONB NOT NULL,
    "expires_at" TIMESTAMPTZ(6) NOT NULL,
    "consumed_at" TIMESTAMPTZ(6),
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "portal_audit_events" (
    "id" UUID NOT NULL PRIMARY KEY,
    "request_id" TEXT,
    "occurred_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "actor_user_id" UUID REFERENCES "portal_users"("id") ON DELETE SET NULL,
    "action" TEXT NOT NULL,
    "target_type" TEXT NOT NULL,
    "target_id" TEXT,
    "system_id" UUID REFERENCES "portal_systems"("id") ON DELETE SET NULL,
    "tenant_id" UUID REFERENCES "portal_tenants"("id") ON DELETE SET NULL,
    "result" "AuditResult" NOT NULL,
    "before_data" JSONB,
    "after_data" JSONB,
    "failure_reason" TEXT,
    "ip_address" TEXT,
    "user_agent" TEXT
);

CREATE INDEX IF NOT EXISTS "portal_audit_events_lookup_idx" ON "portal_audit_events"("occurred_at" DESC, "action", "actor_user_id", "system_id", "tenant_id");

CREATE INDEX IF NOT EXISTS "portal_systems_code_status_idx" ON "portal_systems"("code", "status");
