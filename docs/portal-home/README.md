# 门户首页后台实现

## 对应页面

- `Portal Home`
- 顶部栏的当前用户、租户切换器、管理后台入口
- 常用系统、全部系统、维护中系统分组

## 后台职责

门户首页后台负责把用户、租户、系统目录和权限计算结果组装成可展示的入口列表。它不直接编辑权限，只消费权限域给出的最终结果。

## 主要数据

- `portal_users`：当前用户资料、头像、偏好。
- `portal_tenant_members`：用户所属租户。
- `portal_tenants`：租户状态和名称。
- `portal_systems`：系统名称、图标、描述、入口、状态。
- `portal_tenant_systems`：租户内启用的系统。
- `portal_permission_assignments`：可见和可进入权限。
- `portal_sub_admin_grants`：当前用户在系统内的管理员身份。

## 接口

### `GET /api/portal/home`

查询参数：

- `tenantId`：可选，不传时使用用户默认租户或第一个可用租户。

返回：

```json
{
  "currentTenant": {
    "id": "...",
    "code": "acme",
    "name": "Acme 租户"
  },
  "availableTenants": [],
  "user": {
    "id": "...",
    "displayName": "张三",
    "avatarUrl": null,
    "canEnterAdmin": true
  },
  "groups": [
    {
      "key": "frequent",
      "title": "常用系统",
      "systems": []
    },
    {
      "key": "all",
      "title": "全部系统",
      "systems": []
    },
    {
      "key": "maintenance",
      "title": "维护中系统",
      "systems": []
    }
  ]
}
```

系统项字段：

```json
{
  "systemCode": "northline",
  "name": "Northline",
  "description": "企业数据库问数系统",
  "iconUrl": null,
  "status": "active",
  "visible": true,
  "accessible": true,
  "identityLabel": "租户管理员",
  "tenantLabel": "Acme",
  "permissionSummary": ["northline:query:use"],
  "enterable": true
}
```

### `POST /api/portal/home/frequent`

保存用户常用系统排序和置顶配置，写入 `portal_users.preferences`。

### `POST /api/portal/systems/{systemCode}/enter`

调用认证域签发一次性授权码。门户首页的进入按钮只调用该接口，不自己拼接跳转地址。

## 分组规则

- `frequent`：用户偏好中的置顶系统，且当前仍可见。
- `all`：当前租户下可见且状态为 `active` 或 `onboarding` 的系统。
- `maintenance`：当前租户下可见且状态为 `maintenance` 的系统。
- `disabled` 系统不向普通用户返回，管理员可在系统目录查看。

## 空状态

- 用户没有任何启用租户：返回 `NO_TENANT`，前端展示无租户提示。
- 当前租户没有可见系统：返回空分组和 `ALLOW_PERMISSION_REQUEST` 配置。
- 系统维护中：返回系统卡片但 `enterable=false`。

## 验收

- 普通用户只能看到自己有 `visible=true` 的系统。
- 用户点击可进入系统时，后端返回子系统跳转 URL。
- 多租户用户切换租户后，系统列表和身份徽标同步变化。
- 管理员仍能在门户首页进入自己被授权的业务系统，并额外看到管理后台入口。
