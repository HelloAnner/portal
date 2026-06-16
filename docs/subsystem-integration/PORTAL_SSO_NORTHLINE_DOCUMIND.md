# 门户登录态对接 Northline 与 DocuMind 方案

## 背景

门户定位是企业内部系统统一入口。用户在门户完成登录后，应能从门户首页进入 Northline 或 DocuMind，并由门户把用户身份、租户、目标系统角色和权限上下文下发给对应子系统。

目标功能：

- 用户只在门户登录一次。
- 用户点击子系统入口后，不再看到子系统本地登录页。
- 门户按不同子系统下发不同 `systemRoles` 和 `permissions`。
- Northline 与 DocuMind 保留自身认证体系，独立部署时仍可使用本地登录。
- 门户托管模式下，子系统只信任门户后端换取到的身份上下文，不信任浏览器 URL 中的明文用户信息。

## 当前实现状态

子系统侧配套文档：

- Northline：`/Users/anner/Northline/docs/开发文档/门户统一登录接入方案.md`
- DocuMind：`/Users/anner/DocuMind/docs/access-control/portal-sso-integration.md`

### 门户

门户已有基础的子系统进入链路，但文档和代码之间存在接口命名差异。

已存在能力：

- 门户本地登录使用 `portal_session` HttpOnly Cookie。
- 门户权限矩阵支持按用户、角色、租户配置目标系统的 `visible`、`accessible`、`systemRoles`、`permissions`、`scopes`。
- 前端系统卡片点击后调用：
  - `POST /api/portal/systems/{system_code}/enter`
- 门户后端在进入系统时生成一次性 code，写入 `portal_subsystem_tickets`。
- 门户后端提供子系统换票接口：
  - `POST /api/auth/exchange-ticket`
- code 格式为 `ticket_uuid:secret`，数据库只保存 secret 的 bcrypt hash。
- 换票成功后返回 `context_snapshot`，并把 ticket 标记为已消费。

文档中曾设计的 `POST /api/auth/subsystem-ticket` 当前代码没有同名接口。实际进入入口在 `portal` API 下：

```text
POST /api/portal/systems/{system_code}/enter
```

当前门户下发的上下文快照字段：

```json
{
  "userId": "portal user uuid",
  "username": "zhangsan",
  "displayName": "张三",
  "email": "zhangsan@example.com",
  "avatarUrl": null,
  "tenantId": "portal tenant uuid",
  "tenantCode": "acme",
  "tenantName": "Acme Corp",
  "systemCode": "northline",
  "portalRoles": ["normal-user"],
  "systemRoles": ["enterprise_admin"],
  "permissions": ["northline:chat:ask"],
  "adminScopes": [],
  "issuedAt": 1781590000,
  "expiresAt": 1781590300
}
```

主要缺口：

- `exchange-ticket` 目前没有系统级服务凭证校验，任何拿到 code 和 system code 的服务都可以尝试换票。短期可接受为内网 MVP，正式接入前必须补充子系统 client secret、签名或 mTLS。
- 换票失败没有在 `exchange_ticket` 内写入 `subsystem.ticket.exchange.failure` 审计。
- 门户 docs 中 `subsystem-ticket` 与实际 `/portal/systems/{code}/enter` 命名不一致，需要后续统一。
- 目前没有为 Northline、DocuMind 生成标准 callback URL 和 `.env` 模板的最终字段约定。

### Northline

Northline 已有完整本地登录认证系统。

当前认证模型：

- 本地登录接口：`POST /api/v1/auth/login`
- 当前用户接口：`GET /api/v1/auth/me`
- 刷新接口：`POST /api/v1/auth/refresh`
- 退出接口：`POST /api/v1/auth/logout`
- 前端把 access token 存在 `localStorage`，key 按租户作用域区分。
- 后端使用 HS256 JWT，claims 包含：
  - `sub`
  - `username`
  - `role`
  - `tenant_id`
  - `sid`
  - `exp`
- Redis 中保存 `northline:auth:session:{sid}`。

当前用户上下文：

```rust
CurrentUser {
    id,
    username,
    role,
    tenant_id,
    tenant_slug,
    tenant_name,
    workspace_ids,
    workspace_names,
}
```

角色现状：

- `super_admin`
- `tenant_owner`
- `tenant_admin`
- `enterprise_admin`
- `team_admin`
- `data_admin`
- `user`

Northline 当前代码主要使用单个 `role` 字段做 Agent 和接口权限判断，`tenant_membership.roles` 虽然存在，但登录态和大多数鉴权仍以 `users.role` 为主。

主要缺口：

- 没有 `/auth/portal/callback` 或同等页面/API 接收门户 code。
- 没有调用门户 `/api/auth/exchange-ticket` 的服务端逻辑。
- 没有把门户用户 ID 映射到 Northline 本地 `users.id` 的策略。
- 没有 `portal_managed` 模式开关来旁路本地登录页。
- 门户可下发多个 `systemRoles`，但 Northline 当前运行时更适合接收一个主角色。
- 门户租户 ID 与 Northline `tenants.id` 是否共用同一 UUID 还未约定。

### DocuMind

DocuMind 也已有本地认证、角色路由和租户 ACL，但项目说明文档中仍保留了早期“当前阶段不需要登录页面”的描述，代码已经比说明更完整。

当前认证模型：

- 本地登录接口：`POST /api/v1/auth/login`
- 当前用户接口：`GET /api/v1/me`、`GET /api/v1/auth/me`
- 刷新接口：`POST /api/v1/auth/refresh`
- 退出接口：`POST /api/v1/auth/logout`
- 前端把 access token 存在 `localStorage`，key 为 `documind-auth`。
- 后端使用 HS256 JWT，claims 包含：
  - `sub`
  - `email`
  - `role`
  - `tenant_id`
  - `sid`
  - `exp`
- Redis 中保存 `documind:auth:session:{sid}`。

当前用户上下文：

```rust
CurrentActor {
    user_id,
    tenant_id,
    email,
    name,
    roles,
    permissions,
    allowed_kb_ids,
    is_super_admin,
}
```

角色现状：

- `super_admin`
- `enterprise_admin`
- `team_admin`
- `data_admin`
- `tenant_owner`
- `tenant_admin`
- `user`
- `analyst`
- `end_user`
- `viewer`

DocuMind 的模型天然支持多个角色和权限派生，也支持知识库 ACL：

- 管理角色可访问当前租户所有 active 知识库。
- 普通用户按 `knowledge_base_acl` 中的 role/user 授权计算 `allowed_kb_ids`。

主要缺口：

- 没有 `/auth/portal/callback` 或同等页面/API 接收门户 code。
- 没有调用门户 `/api/auth/exchange-ticket` 的服务端逻辑。
- 没有把门户用户 ID 保存为外部身份源的字段或映射表。
- 没有 `portal_managed` 模式开关来旁路本地登录页。
- 门户下发的 `permissions` 与 DocuMind 本地 `derive_permissions` 的关系需要明确，是直接信任门户权限，还是只用门户角色再本地派生。

## 推荐对接协议

采用 authorization code 风格的后端换票协议，沿用门户当前已有实现。

### 1. 门户进入子系统

```text
Browser
  -> Portal POST /api/portal/systems/{system_code}/enter
  <- { callbackUrl, code, expiresAt }
  -> redirect {callbackUrl}?code={code}
```

门户进入接口必须完成：

1. 校验门户 session。
2. 计算目标系统有效权限。
3. 校验系统 active、租户 active、`accessible=true`。
4. 生成一次性 code。
5. 保存上下文快照。
6. 返回子系统 callback URL。

### 2. 子系统换取上下文

```text
Browser -> Subsystem GET /auth/portal/callback?code=...
Subsystem Backend -> Portal POST /api/auth/exchange-ticket
Portal -> Subsystem Backend context_snapshot
Subsystem Backend -> create local session/JWT
Browser <- redirect subsystem default route
```

子系统请求示例：

```json
{
  "system_code": "northline",
  "code": "ticket_uuid:secret"
}
```

门户返回示例：

```json
{
  "userId": "portal user uuid",
  "username": "zhangsan",
  "displayName": "张三",
  "email": "zhangsan@example.com",
  "tenantId": "portal tenant uuid",
  "tenantCode": "acme",
  "tenantName": "Acme Corp",
  "systemCode": "documind",
  "portalRoles": ["normal-user"],
  "systemRoles": ["enterprise_admin"],
  "permissions": ["documind:chat:ask", "documind:kb:manage"],
  "adminScopes": [],
  "issuedAt": 1781590000,
  "expiresAt": 1781590300
}
```

### 3. 子系统建立本地登录态

子系统拿到门户上下文后，不直接把门户 code 当作本地 token 使用，而是转换为本地 session/JWT。

### 4. 首次进入自动开通

门户是跨系统身份、租户和目标系统角色的来源。Northline、DocuMind 在门户托管模式下收到合法上下文后，必须支持幂等自动开通：

1. 自动注册子系统用户：按 `userId` 查找本地外部身份映射；不存在则创建本地用户，写入门户用户 ID、用户名、显示名、邮箱和头像。
2. 自动分配租户：按 `tenantId` 查找本地租户映射；不存在则用 `tenantId`、`tenantCode`、`tenantName` 创建本地租户。
3. 自动分配角色：按 `systemRoles` 转换为子系统本地角色，写入本地用户主角色或租户成员角色。
4. 自动收敛权限：按 `permissions` 作为权限上限，子系统本地派生权限不能超过门户下发权限。
5. 幂等更新：后续再次进入时更新用户资料、角色、租户成员关系和权限快照，不重复创建用户或租户。

自动开通失败必须拒绝登录并记录审计，不能回退到默认管理员或默认租户。

Northline：

- 查找或创建本地用户。
- 将门户 `userId` 映射到本地 `users.id` 或新增 `external_subject` 字段。
- 将门户 `tenantId` 映射到本地 `tenants.id`；不存在时自动创建本地租户和默认工作区。
- 从 `systemRoles` 中选出一个主角色写入本地 JWT claims。
- 按需要写入 Redis session。
- 前端保存 Northline 本地 access token。

DocuMind：

- 查找或创建本地 `app_user`。
- 将门户 `userId` 映射到 `app_user.sso_subject` 或新增 `external_subject` 字段。
- 将门户 `tenantId` 映射到本地 `tenant.id`；不存在时自动创建本地租户。
- 将 `systemRoles` 写入 `tenant_member.roles` 或只用于本次 session。
- 根据 `systemRoles` 和 `permissions` 计算 `CurrentActor`。
- 前端保存 DocuMind 本地 access token。

## 角色映射建议

门户配置里同一个用户可以对不同系统拥有不同 `systemRoles`。门户只负责下发目标系统角色，不要求两个子系统角色完全一致。

### Northline 角色

| 门户 `systemRoles` | Northline 主角色 | 说明 |
| --- | --- | --- |
| `super_admin` | `super_admin` | 平台级运维，只用于少数可信用户 |
| `tenant_owner` | `tenant_owner` | 租户最高管理者 |
| `tenant_admin` | `tenant_admin` 或 `enterprise_admin` | 建议统一落为 `enterprise_admin`，贴合当前 Northline 企业管理语义 |
| `enterprise_admin` | `enterprise_admin` | 企业管理员，能管理租户内数据源、语义层等 |
| `team_admin` | `team_admin` | 工作区或团队管理员 |
| `data_admin` | `data_admin` | 数据治理和语义层建设角色 |
| `analyst` | `user` | 问数用户 |
| `user` | `user` | 普通问数用户 |

Northline 当前更适合一个主角色。若门户下发多个角色，建议按优先级选择：

```text
super_admin > tenant_owner > enterprise_admin > tenant_admin > team_admin > data_admin > analyst > user
```

### DocuMind 角色

| 门户 `systemRoles` | DocuMind 角色 | 说明 |
| --- | --- | --- |
| `super_admin` | `super_admin` | 全局运维 |
| `tenant_owner` | `tenant_owner` | 租户所有者 |
| `tenant_admin` | `tenant_admin` | 知识库、文档、成员和配置管理 |
| `enterprise_admin` | `enterprise_admin` | 可作为租户管理员兼容角色 |
| `team_admin` | `team_admin` | 管理部分知识库或团队资源 |
| `data_admin` | `data_admin` | 文档处理、检索配置、部分管理能力 |
| `analyst` | `user` 或 `analyst` | 文档问答用户 |
| `user` | `user` | 普通问答用户 |
| `viewer` | `viewer` | 只读知识库用户 |

DocuMind 支持多角色数组，可保留门户下发的多个 `systemRoles`。权限派生可以继续使用本地 `derive_permissions`，但需要用门户下发 `permissions` 做上限约束。

## 权限声明建议

门户 `permissions` 建议使用系统名前缀，避免跨系统冲突。

Northline：

```text
northline:chat:ask
northline:conversation:history
northline:datasource:read
northline:datasource:write
northline:semantic:read
northline:semantic:write
northline:workspace:manage
northline:tenant:manage
northline:audit:read
northline:runtime:manage
```

DocuMind：

```text
documind:chat:ask
documind:knowledge:read
documind:knowledge:write
documind:knowledge:manage
documind:document:upload
documind:document:delete
documind:document:reprocess
documind:member:read
documind:member:write
documind:config:read
documind:config:write
documind:audit:read
documind:model:manage
```

子系统本地权限可以比门户更细，但不能放大门户下发的权限。正式实现时建议采用：

```text
effective_permissions = local_permissions_derived_from_roles ∩ portal_permissions
```

管理员类角色可以配置为门户下发全量权限，避免交集后误伤。

## 身份映射策略

推荐新增外部身份映射，不直接假设三个系统共用用户表。

### Northline

建议新增字段或映射表：

```sql
ALTER TABLE users ADD COLUMN portal_user_id VARCHAR(36);
CREATE UNIQUE INDEX IF NOT EXISTS ux_northline_users_portal_user
ON users(portal_user_id)
WHERE portal_user_id IS NOT NULL;
```

如果 Northline 后续支持一个门户用户进入多个租户，建议使用映射表：

```sql
CREATE TABLE portal_identity_link (
    portal_user_id VARCHAR(36) NOT NULL,
    portal_tenant_id VARCHAR(36),
    local_user_id VARCHAR(36) NOT NULL,
    local_tenant_id VARCHAR(36),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (portal_user_id, portal_tenant_id)
);
```

### DocuMind

DocuMind 已有 `app_user.sso_subject`，可以复用：

```text
auth_provider = 'portal'
sso_subject = portal user id
```

如果需要同时支持多个外部身份源，也建议新增映射表：

```sql
CREATE TABLE external_identity_link (
    provider VARCHAR(32) NOT NULL,
    external_user_id VARCHAR(64) NOT NULL,
    external_tenant_id VARCHAR(64),
    local_user_id UUID NOT NULL REFERENCES app_user(id),
    local_tenant_id UUID REFERENCES tenant(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, external_user_id, external_tenant_id)
);
```

## 配置项建议

门户系统目录：

```text
system.code = northline | documind
system.entry_url = 子系统默认入口
system.callback_url = 子系统 portal callback
integration.auth_mode = authorization_code
integration.token_ttl_seconds = 300
```

Northline `.env`：

```env
SYSTEM_CODE=northline
PORTAL_MANAGED=true
PORTAL_AUTH_ENABLED=true
PORTAL_BASE_URL=http://localhost:8080
PORTAL_EXCHANGE_ENDPOINT=/api/auth/exchange-ticket
PORTAL_AUTH_CALLBACK=/auth/portal/callback
PORTAL_CLIENT_ID=northline
PORTAL_CLIENT_SECRET=change-me
```

DocuMind `.env`：

```env
SYSTEM_CODE=documind
PORTAL_MANAGED=true
PORTAL_AUTH_ENABLED=true
PORTAL_BASE_URL=http://localhost:8080
PORTAL_EXCHANGE_ENDPOINT=/api/auth/exchange-ticket
PORTAL_AUTH_CALLBACK=/auth/portal/callback
PORTAL_CLIENT_ID=documind
PORTAL_CLIENT_SECRET=change-me
```

`PORTAL_CLIENT_SECRET` 需要门户和子系统双边保存。门户当前表没有 client secret 字段，正式实现前应在 `portal_integration_configs` 增加 `client_id`、`client_secret_hash` 或等价服务凭证配置。

## 需要改造的接口

### 门户

短期必须补齐：

- `POST /api/auth/exchange-ticket` 增加子系统服务认证。
- 换票失败写审计 `subsystem.ticket.exchange.failure`。
- 统一文档接口命名，选择保留实际接口 `/api/portal/systems/{system_code}/enter`，或补一个 `/api/auth/subsystem-ticket` 兼容别名。
- `portal_integration_configs` 增加子系统服务凭证配置。
- 管理端为 Northline、DocuMind 提供默认角色和权限声明模板。

可选增强：

- 支持 JWT 模式。门户直接签发短期 JWT 给子系统，子系统使用门户公钥验签。但 authorization code 更适合浏览器跳转，避免在 URL 中暴露完整身份信息。

### Northline

必须新增：

- `GET /auth/portal/callback?code=...` 页面或 API。
- 后端调用门户 `/api/auth/exchange-ticket`。
- `PortalContext` 结构体，解析门户上下文。
- 门户身份到本地用户、租户、工作区的映射逻辑。
- 根据门户 `systemRoles` 选择 Northline 主角色。
- 门户托管模式下，访问 `/login` 时重定向到门户或显示禁用本地登录提示。

建议新增：

- `portal_user_id` 或 `portal_identity_link`。
- `PORTAL_MANAGED`、`PORTAL_AUTH_ENABLED`、`PORTAL_BASE_URL`、`PORTAL_CLIENT_SECRET` 配置。
- 审计事件：`portal.login.success`、`portal.login.failure`。

### DocuMind

必须新增：

- `GET /auth/portal/callback?code=...` 页面或 API。
- 后端调用门户 `/api/auth/exchange-ticket`。
- `PortalContext` 结构体，解析门户上下文。
- 使用 `app_user.sso_subject` 或映射表绑定门户用户。
- 根据门户 `systemRoles` 建立 `CurrentActor`。
- 门户托管模式下，访问 `/login` 时重定向到门户或显示禁用本地登录提示。

建议新增：

- `auth_provider='portal'` 的用户创建和更新逻辑。
- 门户 `permissions` 与本地 `derive_permissions` 的交集计算。
- 门户 `adminScopes` 到知识库 ACL 或管理范围的转换规则。

## 推荐实施顺序

1. 统一门户文档和实际接口，明确 `/api/portal/systems/{system_code}/enter` 是浏览器进入入口。
2. 门户为 `exchange-ticket` 增加子系统服务认证和失败审计。
3. 在 Northline 实现 portal callback，先完成最小可用的换票和本地 JWT 签发。
4. 在 DocuMind 实现 portal callback，复用 `CurrentActor` 和 `derive_permissions`。
5. 在门户管理端补齐 Northline、DocuMind 的默认角色和权限模板。
6. 对两个子系统增加 `PORTAL_MANAGED=true` 下的本地登录旁路。
7. 增加端到端测试：门户登录后分别进入 Northline 和 DocuMind，验证角色不同、权限不同、重复 code 被拒绝。

## 验收标准

- 门户普通用户登录后，可以直接进入已授权的 Northline 和 DocuMind。
- 同一个门户用户在 Northline 可是 `enterprise_admin`，在 DocuMind 可是 `viewer` 或 `user`。
- 用户没有 `accessible=true` 时，门户不会签发 code。
- 子系统拿到 code 后只能服务端换票，浏览器 URL 中不携带用户、角色、权限明文。
- code 过期、重复使用、system code 不匹配都会失败。
- 子系统本地登录在独立部署模式可用，在门户托管模式被关闭或旁路。
- 子系统本地权限不会超过门户下发权限。
