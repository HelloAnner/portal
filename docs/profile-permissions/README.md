# 我的资料与我的权限后台实现

## 对应页面

- `My Profile`
- `My Permissions`
- `Northline 权限详情` 抽屉

## 后台职责

个人中心分为资料维护和权限解释。资料维护写用户自己的展示信息和偏好；权限解释只读展示最终权限、权限来源和即将下发给子系统的上下文摘要。

## 数据表

- `portal_users`
- `portal_tenants`
- `portal_tenant_members`
- `portal_roles`
- `portal_user_roles`
- `portal_systems`
- `portal_permission_assignments`
- `portal_sub_admin_grants`

## 接口

### `GET /api/profile`

返回我的资料页左侧摘要和右侧表单数据：

```json
{
  "username": "zhangsan",
  "displayName": "张三",
  "email": "zhangsan@example.com",
  "phone": "13800000000",
  "organizationPath": "研发/平台",
  "status": "active",
  "avatarUrl": null,
  "preferences": {
    "defaultTenantId": "...",
    "homeGroupMode": "frequent_first",
    "showPermissionRequest": true
  }
}
```

### `PATCH /api/profile`

允许用户修改：

- `displayName`
- `avatarUrl`
- `defaultTenantId`
- `preferences`

邮箱、手机号是否允许自助修改由企业策略决定。默认只读，管理员可在用户管理页修改。

### `POST /api/profile/avatar`

接收头像上传，保存对象存储地址或本地文件地址。返回裁剪后的 `avatarUrl`。实现时需要校验文件类型、大小和安全扫描结果。

### `GET /api/me/permissions`

返回我的权限页列表：

```json
{
  "portalRoles": ["user"],
  "tenants": [],
  "systems": [
    {
      "systemCode": "northline",
      "systemName": "Northline",
      "tenantName": "Acme",
      "identity": "tenant-admin",
      "visible": true,
      "accessible": true,
      "sourceSummary": "租户授权",
      "scopeSummary": "系统级、模块级"
    }
  ]
}
```

### `GET /api/me/permissions/{systemCode}`

返回抽屉详情：

```json
{
  "systemCode": "northline",
  "tenantId": "...",
  "identity": "tenant-admin",
  "sources": [
    {
      "type": "tenant",
      "name": "Acme 租户授权"
    }
  ],
  "permissions": [
    "northline:query:use",
    "northline:datasource:manage"
  ],
  "adminScopes": [
    {
      "scopeType": "module",
      "scopeCode": "datasource"
    }
  ],
  "contextPreview": {}
}
```

## 权限解释规则

- 用户只能查看自己的权限来源，不能查看其他用户的权限。
- 权限来源需要区分用户直配、角色继承、租户授权、子管理员授权。
- `contextPreview` 是只读预览，不作为进入子系统凭证。
- 已过期的授权可在详情中显示为历史，但不进入最终上下文。

## 验收

- 显示名称修改后，门户首页和子系统身份上下文都使用新名称。
- 默认租户修改后，下次打开门户首页默认选中新租户。
- 我的权限能解释 Northline、DocuMind 的身份、权限声明和管理范围。
- 普通用户不能通过接口修改自己的门户角色或子管理员身份。
