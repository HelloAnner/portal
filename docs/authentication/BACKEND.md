# 登录与认证后台实现

## 对应页面

- `Login Page`
- `No Permission` 中的重新登录和回跳
- 门户首页进入子系统前的凭证签发

## 后台职责

认证域负责用户登录、会话维护、退出、回跳地址校验、短时子系统凭证签发和子系统凭证换取。它不负责配置用户能访问哪些系统，权限判断由权限域提供计算结果。

## 配置

```env
SESSION_COOKIE_NAME=portal_session
SESSION_TTL_SECONDS=28800
REMEMBER_ME_TTL_SECONDS=2592000
PORTAL_ISSUER=http://localhost:8080
PORTAL_TOKEN_TTL_SECONDS=300
PORTAL_JWT_PRIVATE_KEY_PATH=./config/portal-private-key.pem
PORTAL_JWT_PUBLIC_KEY_PATH=./config/portal-public-key.pem
```

## 数据表

- `portal_users`：读取账号状态、密码摘要、基础资料。
- `portal_sessions`：保存会话摘要、过期时间、撤销状态。
- `portal_subsystem_tickets`：保存一次性授权码摘要和上下文快照。
- `portal_audit_events`：记录登录、退出、凭证签发、凭证换取失败。

## 接口

### `POST /api/auth/login`

请求：

```json
{
  "username": "zhangsan",
  "password": "******",
  "rememberMe": true,
  "redirectTo": "/"
}
```

行为：

- 校验账号是否存在、状态是否为 `active`。
- 校验密码或企业 SSO 返回的外部身份。
- 创建 `portal_sessions`，只把 session 摘要写入库。
- 写入 `login.success` 或 `login.failure` 审计。
- 登录成功后返回安全的回跳地址。

### `GET /api/auth/me`

返回当前会话用户、门户角色、默认租户和管理后台入口可见性。前端顶部栏使用该接口渲染当前用户和租户切换器。

### `POST /api/auth/logout`

撤销当前 session，写入 `logout.success` 审计。

### `POST /api/auth/subsystem-ticket`

请求：

```json
{
  "systemCode": "northline",
  "tenantId": "..."
}
```

行为：

- 校验用户会话有效。
- 调用权限域计算目标系统最终上下文。
- 校验系统状态、租户状态、用户可进入权限。
- 生成 5 分钟内有效的一次性授权码。
- 将身份上下文快照写入 `portal_subsystem_tickets`。
- 返回子系统回调地址和授权码。

### `POST /api/auth/exchange-ticket`

子系统后端调用。请求必须携带系统级服务凭证或签名。

请求：

```json
{
  "systemCode": "northline",
  "code": "one_time_code"
}
```

行为：

- 校验系统编码与授权码快照一致。
- 校验授权码未过期、未消费。
- 标记 `consumed_at`。
- 返回上下文快照，或返回签名 JWT。

## 关键规则

- 禁用、归档、待激活用户不能登录，也不能换取子系统凭证。
- `redirectTo` 只允许站内路径或已登记的子系统回调，禁止开放重定向。
- 一次性授权码只保存哈希，明文只返回一次。
- 用户可见系统不代表可进入系统，签发子系统凭证必须检查 `accessible=true`。
- 维护中和停用系统不能签发普通用户凭证；超级管理员调试入口也必须写审计。

## 与子系统协作

Northline、DocuMind 在门户托管模式下收到授权码后，必须从后端调用 `/api/auth/exchange-ticket`。子系统不能信任浏览器 URL 中的用户 ID、角色或权限明文参数。

## 验收

- 登录成功后 `GET /api/auth/me` 能拿到顶部栏所需用户信息。
- 密码错误、账号禁用、会话过期都有清晰错误码。
- 点击门户首页系统入口时能生成短时授权码并跳转。
- 授权码重复使用、过期、系统编码不匹配时被拒绝并写入审计。
