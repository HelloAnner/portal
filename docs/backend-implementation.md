# 门户后台实现总览

## 目标

本文档把 `portal.pen` 中的页面设计落到可实现的后台边界。门户后端采用 Rust 实现，和 Northline、DocuMind 的 `apps/api-rs` 服务实现逻辑保持一致。门户后台以 `.env` 配置启动，默认连接 PostgreSQL，并允许部署者配置 PostgreSQL 地址、数据库名和 schema。所有页面都从同一套身份、租户、系统目录、权限、接入和审计数据中取数，避免 UI 看起来完整但后端无法闭环。

## 运行配置

门户服务启动时读取 `.env`。未显式配置时使用本地 PostgreSQL 默认值，生产环境必须覆盖连接信息、密钥和域名。

```env
APP_ENV=development
APP_PORT=8080
APP_BASE_URL=http://localhost:8080

PG_HOST=127.0.0.1
PG_PORT=5432
PG_DATABASE=portal
PG_USER=portal
PG_PASSWORD=portal
PG_SCHEMA=portal
PG_SSL=false

SESSION_COOKIE_NAME=portal_session
SESSION_TTL_SECONDS=28800
REMEMBER_ME_TTL_SECONDS=2592000

PORTAL_ISSUER=http://localhost:8080
PORTAL_TOKEN_TTL_SECONDS=300
PORTAL_JWT_PRIVATE_KEY_PATH=./config/portal-private-key.pem
PORTAL_JWT_PUBLIC_KEY_PATH=./config/portal-public-key.pem

AUDIT_RETENTION_DAYS=365
ALLOW_PERMISSION_REQUEST=true
```

实现要求：

- 所有 SQL 连接在启动后执行 `set search_path to ${PG_SCHEMA}, public`，并在迁移脚本中使用可配置 schema。
- `PG_SCHEMA` 不存在时，开发环境可自动创建；生产环境建议由迁移任务创建。
- 后台不依赖前端 URL 参数作为可信身份来源，所有进入子系统的身份上下文由服务端签发或换取。
- 密码、令牌、一次性授权码只保存摘要或不可逆哈希。

## 服务模块

Rust 后端建议按领域拆分为以下模块：

| 模块 | 对应页面 | 职责 |
| --- | --- | --- |
| AuthService | 登录页、无权限页 | 登录、会话、退出、短时凭证、回跳 |
| PortalHomeService | 门户首页 | 系统入口聚合、租户上下文、进入子系统 |
| ProfileService | 我的资料 | 用户自助资料、头像、偏好 |
| PermissionViewService | 我的权限 | 最终权限解释、来源追踪、详情抽屉 |
| AdminOverviewService | 管理后台概览 | 指标、风险事项、近期审计 |
| UserService | 用户管理 | 用户资料、状态、角色、租户关系 |
| TenantService | 租户管理 | 租户、成员、租户内系统启用 |
| RoleService | 角色管理 | 门户角色、角色成员、角色授权 |
| SystemCatalogService | 系统目录 | 子系统元数据、入口、状态、身份层级 |
| PermissionConfigService | 权限配置 | 可见、可进入、身份、权限声明、预览 |
| SubAdminService | 子系统管理员 | 管理员身份、范围、生效/失效 |
| IntegrationService | 子系统接入 | 接入配置、回调、校验方式、`.env` 示例 |
| AuditService | 安全与审计 | 审计写入、查询、导出、风险事件 |

## 数据模型

核心数据表使用 PostgreSQL。字段名可按技术栈调整，但语义必须保留。

```sql
create table portal_users (
  id uuid primary key,
  username text not null unique,
  password_hash text,
  display_name text not null,
  email text,
  phone text,
  avatar_url text,
  organization_path text,
  status text not null check (status in ('active', 'disabled', 'pending', 'archived')),
  default_tenant_id uuid,
  preferences jsonb not null default '{}'::jsonb,
  last_login_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table portal_roles (
  id uuid primary key,
  code text not null unique,
  name text not null,
  role_type text not null check (role_type in ('super_admin', 'normal', 'subsystem_admin', 'custom')),
  description text,
  is_builtin boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table portal_user_roles (
  user_id uuid not null references portal_users(id),
  role_id uuid not null references portal_roles(id),
  tenant_id uuid,
  created_at timestamptz not null default now(),
  primary key (user_id, role_id, tenant_id)
);

create table portal_tenants (
  id uuid primary key,
  code text not null unique,
  name text not null,
  status text not null check (status in ('active', 'disabled', 'archived')),
  description text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table portal_tenant_members (
  tenant_id uuid not null references portal_tenants(id),
  user_id uuid not null references portal_users(id),
  member_status text not null default 'active',
  joined_at timestamptz not null default now(),
  primary key (tenant_id, user_id)
);

create table portal_systems (
  id uuid primary key,
  code text not null unique,
  name text not null,
  description text,
  category text,
  icon_url text,
  entry_url text not null,
  callback_url text,
  status text not null check (status in ('active', 'disabled', 'onboarding', 'maintenance')),
  portal_managed boolean not null default true,
  auth_enabled boolean not null default true,
  supports_sub_admin boolean not null default false,
  supported_identity_levels jsonb not null default '[]'::jsonb,
  supported_permissions jsonb not null default '[]'::jsonb,
  supported_scopes jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table portal_tenant_systems (
  tenant_id uuid not null references portal_tenants(id),
  system_id uuid not null references portal_systems(id),
  enabled boolean not null default true,
  created_at timestamptz not null default now(),
  primary key (tenant_id, system_id)
);

create table portal_permission_assignments (
  id uuid primary key,
  subject_type text not null check (subject_type in ('user', 'role', 'tenant')),
  subject_id uuid not null,
  tenant_id uuid references portal_tenants(id),
  system_id uuid not null references portal_systems(id),
  visible boolean not null default false,
  accessible boolean not null default false,
  system_roles text[] not null default '{}',
  permissions text[] not null default '{}',
  scopes jsonb not null default '[]'::jsonb,
  source_note text,
  starts_at timestamptz,
  expires_at timestamptz,
  created_by uuid references portal_users(id),
  updated_by uuid references portal_users(id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table portal_sub_admin_grants (
  id uuid primary key,
  user_id uuid not null references portal_users(id),
  tenant_id uuid references portal_tenants(id),
  system_id uuid not null references portal_systems(id),
  admin_type text not null check (admin_type in ('system', 'tenant', 'module', 'resource', 'organization')),
  scopes jsonb not null default '[]'::jsonb,
  status text not null check (status in ('active', 'inactive', 'expired')),
  reason text,
  starts_at timestamptz not null default now(),
  expires_at timestamptz,
  created_by uuid references portal_users(id),
  updated_by uuid references portal_users(id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table portal_integration_configs (
  system_id uuid primary key references portal_systems(id),
  issuer text not null,
  auth_mode text not null check (auth_mode in ('jwt', 'authorization_code')),
  token_ttl_seconds integer not null default 300,
  public_key text,
  verify_endpoint text,
  env_template jsonb not null default '{}'::jsonb,
  last_check_at timestamptz,
  last_check_result jsonb,
  updated_at timestamptz not null default now()
);

create table portal_sessions (
  id uuid primary key,
  user_id uuid not null references portal_users(id),
  session_hash text not null unique,
  remember_me boolean not null default false,
  expires_at timestamptz not null,
  revoked_at timestamptz,
  ip_address inet,
  user_agent text,
  created_at timestamptz not null default now()
);

create table portal_subsystem_tickets (
  id uuid primary key,
  code_hash text not null unique,
  user_id uuid not null references portal_users(id),
  tenant_id uuid references portal_tenants(id),
  system_id uuid not null references portal_systems(id),
  context_snapshot jsonb not null,
  expires_at timestamptz not null,
  consumed_at timestamptz,
  created_at timestamptz not null default now()
);

create table portal_audit_events (
  id uuid primary key,
  request_id text,
  occurred_at timestamptz not null default now(),
  actor_user_id uuid references portal_users(id),
  action text not null,
  target_type text not null,
  target_id text,
  system_id uuid references portal_systems(id),
  tenant_id uuid references portal_tenants(id),
  result text not null check (result in ('success', 'failure')),
  before_data jsonb,
  after_data jsonb,
  failure_reason text,
  ip_address inet,
  user_agent text
);
```

建议索引：

- `portal_permission_assignments(subject_type, subject_id, tenant_id, system_id)`
- `portal_sub_admin_grants(user_id, tenant_id, system_id, status)`
- `portal_audit_events(occurred_at desc, action, actor_user_id, system_id, tenant_id)`
- `portal_sessions(user_id, expires_at)`
- `portal_systems(code, status)`

## 权限计算

用户最终权限由四类来源合并：

1. 用户直配权限。
2. 用户拥有角色带来的权限。
3. 用户所在租户带来的系统启用和默认权限。
4. 子管理员授权带来的管理员身份和范围。

合并规则：

- `visible=false` 的显式撤销优先级高于角色默认授权。
- `accessible=true` 必须同时满足用户状态启用、租户状态启用、系统状态启用。
- `system_roles`、`permissions` 做去重合并。
- `scopes` 按 `scopeType + scopeCode` 去重。
- 子管理员授权过期后不进入最终上下文。
- 维护中系统可见但不可进入；停用系统普通用户不可见或不可进入。

## 子系统跳转

门户进入子系统时支持两种实现：

1. JWT 模式：门户签发短时 JWT，浏览器跳转到子系统 `callback_url`，子系统使用门户公钥校验。
2. 授权码模式：门户生成一次性 `code`，浏览器跳转子系统，子系统后端调用门户接口换取身份上下文。

推荐默认使用授权码模式，减少浏览器暴露身份上下文。

## 接口风格

后台接口统一使用 `/api` 前缀。列表接口都支持分页、筛选、排序，管理类写操作必须写审计。跨子系统可信接口使用服务端凭证或签名校验。

通用响应：

```json
{
  "data": {},
  "requestId": "req_...",
  "error": null
}
```

错误码：

- `AUTH_REQUIRED`
- `PERMISSION_DENIED`
- `SYSTEM_DISABLED`
- `TENANT_DISABLED`
- `INVALID_SUBSYSTEM_TICKET`
- `VALIDATION_FAILED`
- `CONFLICT`

## 页面到后端文档映射

| 设计页面 | 后端文档 |
| --- | --- |
| Login Page | `docs/authentication/BACKEND.md` |
| Portal Home | `docs/portal-home/README.md` |
| My Profile / My Permissions | `docs/profile-permissions/README.md` |
| Admin Overview | `docs/admin-overview/README.md` |
| User Management | `docs/user-role-management/BACKEND.md` |
| Tenant Management | `docs/tenant-management/README.md` |
| System Catalog | `docs/system-catalog/BACKEND.md` |
| Permission Config | `docs/permission-configuration/BACKEND.md` |
| Subsystem Admin | `docs/sub-admin/BACKEND.md` |
| Subsystem Integration | `docs/subsystem-integration/BACKEND.md` |
| Role Management | `docs/role-management/README.md` |
| Security Audit | `docs/security-audit/BACKEND.md` |
| No Permission | `docs/no-permission/README.md` |

## 验收闭环

- 启动时能通过 `.env` 连接默认 PostgreSQL。
- 修改 `PG_HOST`、`PG_PORT`、`PG_DATABASE`、`PG_USER`、`PG_PASSWORD`、`PG_SCHEMA` 后无需改代码即可连接其他库和 schema。
- 普通用户能登录、查看门户首页、进入授权系统、查看个人资料和权限。
- 管理员能维护用户、租户、角色、系统、权限、子管理员和接入配置。
- Northline、DocuMind 能以门户下发的用户 ID、租户、角色、权限和管理员范围初始化访问上下文。
- 所有登录、跳转、授权和接入变更都有审计记录。
