# 用户管理后台实现

## 对应页面

- `User Management`
- 用户详情抽屉
- 用户批量启用、禁用、授权

## 后台职责

用户管理维护企业用户基础资料、账号状态、门户角色、所属租户和可访问系统摘要。角色定义本身由角色管理域维护，本域只负责用户和角色、租户、系统授权之间的绑定视图。

## 数据表

- `portal_users`
- `portal_roles`
- `portal_user_roles`
- `portal_tenants`
- `portal_tenant_members`
- `portal_permission_assignments`
- `portal_sub_admin_grants`
- `portal_audit_events`

## 接口

### `GET /api/admin/users`

筛选：

- `keyword`
- `status`
- `roleCode`
- `tenantId`
- `systemCode`
- `organizationPath`

返回用户表格字段：

```json
{
  "items": [
    {
      "id": "...",
      "username": "zhangsan",
      "displayName": "张三",
      "email": "zhangsan@example.com",
      "organizationPath": "研发/平台",
      "status": "active",
      "portalRoles": ["super_admin"],
      "tenantCount": 2,
      "accessibleSystemCount": 3,
      "lastLoginAt": "2026-06-14T10:00:00Z"
    }
  ],
  "total": 1
}
```

### `POST /api/admin/users`

创建用户。必须校验用户名唯一、邮箱格式、初始状态。若传入初始角色或租户，同时写入绑定表并记录审计。

### `GET /api/admin/users/{userId}`

返回详情抽屉的标签页数据：

- 基础资料。
- 租户列表。
- 系统权限摘要。
- 子系统身份。
- 该用户相关审计记录。

### `PATCH /api/admin/users/{userId}`

编辑显示名、邮箱、手机号、组织、状态等字段。禁用用户时：

- 撤销或标记失效活跃 session。
- 后续不再签发子系统凭证。
- 写入高风险审计。

### `PUT /api/admin/users/{userId}/roles`

替换用户门户角色绑定。不能移除系统最后一个超级管理员。

### `PUT /api/admin/users/{userId}/tenants`

替换或增量修改用户所属租户。移出租户时，需要同步计算该租户下系统授权是否失效。

### `POST /api/admin/users/batch`

批量操作：

- `enable`
- `disable`
- `assignRole`
- `removeRole`
- `addToTenant`
- `removeFromTenant`

批量写操作必须逐条记录结果，允许部分成功，并把批次号写入审计 `request_id`。

## 业务规则

- `disabled`、`archived` 用户不可登录、不可进入任何托管子系统。
- 用户名创建后默认不可修改，避免子系统审计追踪混乱。
- 门户用户 ID 是子系统识别用户的唯一稳定外键。
- 管理员重置认证信息后，旧 session 可配置为立即失效。

## 验收

- 用户表格字段能覆盖设计中的筛选、列表和详情抽屉。
- 禁用用户后，该用户当前 session 与子系统进入能力都失效。
- 用户详情能展示租户、系统权限、子系统身份和审计记录四类信息。
