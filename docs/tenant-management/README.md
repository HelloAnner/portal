# 租户管理后台实现

## 对应页面

- `Tenant Management`
- 租户详情抽屉
- 租户成员和租户内系统授权

## 后台职责

租户域负责管理企业内部的租户边界，并维护用户与租户、租户与系统、租户内管理员之间的关系。租户不是子系统业务资源本身，但会作为门户下发身份上下文的重要范围。

## 数据表

- `portal_tenants`
- `portal_tenant_members`
- `portal_tenant_systems`
- `portal_users`
- `portal_systems`
- `portal_permission_assignments`
- `portal_sub_admin_grants`

## 接口

### `GET /api/admin/tenants`

返回租户表格：

```json
{
  "items": [
    {
      "id": "...",
      "code": "acme",
      "name": "Acme 租户",
      "status": "active",
      "memberCount": 42,
      "enabledSystemCount": 3,
      "tenantAdminCount": 2,
      "createdAt": "2026-06-14T10:00:00Z"
    }
  ],
  "total": 1
}
```

### `POST /api/admin/tenants`

创建租户。租户编码唯一，建议使用小写字母、数字和短横线。

### `GET /api/admin/tenants/{tenantId}`

返回详情抽屉：

- 基础信息。
- 成员列表。
- 启用系统列表。
- 每个系统的租户管理员。
- 资源范围说明。

### `PATCH /api/admin/tenants/{tenantId}`

编辑租户名称、描述、状态。停用租户时：

- 当前租户下所有普通用户不能进入子系统。
- 子管理员授权在最终权限计算中失效。
- 写入高风险审计。

### `PUT /api/admin/tenants/{tenantId}/members`

添加或移除成员。移除成员时保留历史审计，不删除用户本身。

### `PUT /api/admin/tenants/{tenantId}/systems`

配置租户可用系统。只有租户和系统均启用时，最终上下文才允许进入。

## 租户内身份

租户管理员不是独立角色表中的固定角色，而是用户在某个租户、某个系统中的身份组合：

- 租户成员：只能使用被授权系统。
- 租户管理员：在该租户范围内管理指定系统。
- 子系统超级管理员：在目标系统内拥有更高管理声明，但仍可受租户范围约束。

## 验收

- 租户详情能展示成员数量、启用系统、租户管理员。
- 停用租户后，该租户下系统入口不可进入。
- 用户切换门户首页租户时，系统入口和身份随租户变化。
