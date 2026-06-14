# 系统目录后台实现

## 对应页面

- `System Catalog`
- `Northline 配置` 抽屉
- 新增系统四步配置流程

## 后台职责

系统目录维护子系统入口元数据，是门户首页展示和子系统跳转的基础。它负责系统是什么、在哪里、当前状态如何、支持哪些身份层级和权限声明，不直接给具体用户授权。

## 数据表

- `portal_systems`
- `portal_tenant_systems`
- `portal_integration_configs`
- `portal_permission_assignments`
- `portal_sub_admin_grants`

## 接口

### `GET /api/admin/systems`

筛选：

- `keyword`
- `status`
- `category`
- `portalManaged`
- `supportsSubAdmin`

返回系统表格字段：

```json
{
  "items": [
    {
      "id": "...",
      "code": "northline",
      "name": "Northline",
      "category": "数据",
      "entryUrl": "https://northline.example.com",
      "status": "active",
      "portalManaged": true,
      "supportsSubAdmin": true,
      "updatedAt": "2026-06-14T10:00:00Z"
    }
  ],
  "total": 1
}
```

### `POST /api/admin/systems`

新增系统，分步表单最终提交：

- 基础信息：名称、编码、分类、图标、描述。
- 认证配置：入口地址、回调地址、校验方式。
- 角色声明：支持的身份层级、权限声明、范围类型。
- 发布确认：状态、是否门户托管认证。

### `GET /api/admin/systems/{systemId}`

返回详情抽屉字段：

- 系统名称。
- 系统编码。
- 入口地址。
- 回调地址。
- 凭证校验方式。
- 支持身份层级。
- 支持权限声明。
- 已启用租户数量。
- 已授权用户数量。

### `PATCH /api/admin/systems/{systemId}`

编辑系统元数据。修改 `code` 需要禁止或做迁移，因为子系统 `.env` 中的 `SYSTEM_CODE` 依赖该值。

### `POST /api/admin/systems/{systemId}/status`

状态切换：

- `active`：可按权限进入。
- `maintenance`：用户可见但不可进入。
- `disabled`：普通用户不可见或不可进入。
- `onboarding`：仅管理员可见。

## 内置系统

初始迁移应至少种子化：

- `northline`：支持普通用户、租户管理员、超级管理员；权限包括问数使用、数据源管理、问数配置管理。
- `documind`：支持普通用户、租户管理员、超级管理员；权限包括文档检索、知识库管理、文档上传、RAG 配置管理。

## 验收

- 系统目录能驱动门户首页系统卡片展示。
- 系统停用或维护后，进入子系统接口拒绝签发凭证。
- 系统支持的权限声明能被权限配置和子管理员范围选择器复用。
