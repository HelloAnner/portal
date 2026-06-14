# 子系统管理员后台实现

## 对应页面

- `Subsystem Admin`
- `新建子系统管理员` 抽屉向导

## 后台职责

子管理员域专门管理用户在某个子系统、租户、模块或资源范围内的管理员身份。它把管理员身份作为结构化授权保存，并在进入子系统时合并到最终上下文。

## 数据表

- `portal_sub_admin_grants`
- `portal_users`
- `portal_tenants`
- `portal_systems`
- `portal_audit_events`

## 接口

### `GET /api/admin/sub-admins`

筛选：

- `userId`
- `systemCode`
- `tenantId`
- `adminType`
- `status`
- `expiresBefore`

返回列表：

```json
{
  "items": [
    {
      "id": "...",
      "userDisplayName": "张三",
      "systemName": "Northline",
      "tenantName": "Acme",
      "adminType": "module",
      "scopeSummary": "datasource",
      "status": "active",
      "startsAt": "...",
      "expiresAt": null,
      "createdByName": "管理员"
    }
  ],
  "total": 1
}
```

### `POST /api/admin/sub-admins`

新建子管理员授权。抽屉向导字段：

- 选择用户。
- 选择系统。
- 选择租户。
- 选择身份层级。
- 选择管理范围。
- 确认生效时间、失效时间和原因。

### `PATCH /api/admin/sub-admins/{grantId}`

修改管理范围、有效期、状态。撤销管理员身份时可选择 `revokeActiveTickets=true`。

### `GET /api/admin/sub-admins/scope-options`

根据系统返回范围选择器数据：

- Northline：数据源、问数配置、查询管理等模块。
- DocuMind：知识库、文档库、文档上传、RAG 配置等模块。

范围选项来自 `portal_systems.supported_scopes`，必要时可由子系统接入接口同步。

## 下发上下文

最终权限中生成：

```json
{
  "admin": true,
  "adminType": "module",
  "adminScopes": [
    {
      "scopeType": "module",
      "scopeCode": "datasource"
    }
  ]
}
```

## 验收

- 同一用户可同时拥有 Northline 和 DocuMind 的不同管理员范围。
- 子管理员失效或撤销后，不再进入下发上下文。
- 高风险撤销能触发已有凭证失效策略。
