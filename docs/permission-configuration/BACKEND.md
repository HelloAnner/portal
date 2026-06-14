# 权限配置后台实现

## 对应页面

- `Permission Config`
- 按用户、租户、系统、角色视角的权限矩阵
- 最终下发上下文预览

## 后台职责

权限配置域维护授权关系，并提供最终权限计算。它是门户能否展示系统、能否进入系统、进入后是什么身份的核心来源。

## 数据表

- `portal_permission_assignments`
- `portal_users`
- `portal_roles`
- `portal_user_roles`
- `portal_tenants`
- `portal_systems`
- `portal_sub_admin_grants`

## 接口

### `GET /api/admin/permissions/matrix`

查询参数：

- `view=user|tenant|system|role`
- `userId`
- `tenantId`
- `systemId`
- `roleId`

返回矩阵行：

```json
{
  "rows": [
    {
      "subjectType": "user",
      "subjectId": "...",
      "tenantId": "...",
      "systemId": "...",
      "visible": true,
      "accessible": true,
      "systemRoles": ["tenant-admin"],
      "permissions": ["northline:query:use"],
      "scopes": [],
      "source": "direct"
    }
  ]
}
```

### `PUT /api/admin/permissions/assignments`

批量保存权限矩阵修改。请求中每一项都包含完整目标维度，后端做 upsert。

### `POST /api/admin/permissions/preview`

保存前差异预览：

```json
{
  "userId": "...",
  "tenantId": "...",
  "systemCode": "northline",
  "draftChanges": []
}
```

返回：

- 当前最终上下文。
- 修改后最终上下文。
- 新增权限。
- 撤销权限。
- 高风险提示。

### `GET /api/admin/permissions/effective`

用于排查某个用户最终会下发给子系统的上下文。

## 计算规则

1. 检查用户、租户、系统状态。
2. 读取用户直配授权。
3. 读取用户角色授权。
4. 读取租户默认授权。
5. 合并子管理员授权。
6. 根据系统支持的声明过滤无效权限。
7. 输出 `visible`、`accessible`、`systemRoles`、`permissions`、`adminScopes`。

## 高风险变更

以下变更必须二次确认并写入高风险审计：

- 授予或撤销 `super-admin`。
- 撤销子系统管理员身份。
- 将系统 `accessible` 从 true 改为 false。
- 批量影响超过阈值的授权变更。
- 选择立即失效已有子系统访问凭证。

## 验收

- `visible` 和 `accessible` 可独立配置。
- 权限矩阵能按用户、租户、系统、角色四个视角加载。
- 保存前能看到最终上下文差异。
- 权限变更后，用户下一次进入子系统时收到新上下文。
