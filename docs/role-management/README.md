# 角色管理后台实现

## 对应页面

- `Role Management`
- `租户管理员` 角色详情抽屉
- 角色权限矩阵入口

## 后台职责

角色域管理门户角色和角色绑定的默认权限，用于减少逐用户授权。角色本身描述身份和默认授权模板，最终是否能进入某系统仍需结合用户状态、租户状态、系统状态和权限计算。

## 数据表

- `portal_roles`
- `portal_user_roles`
- `portal_permission_assignments`
- `portal_users`
- `portal_tenants`
- `portal_systems`

## 接口

### `GET /api/admin/roles`

返回角色列表：

```json
{
  "items": [
    {
      "id": "...",
      "code": "tenant-admin",
      "name": "租户管理员",
      "roleType": "custom",
      "memberCount": 12,
      "boundSystemCount": 2,
      "updatedAt": "2026-06-14T10:00:00Z"
    }
  ],
  "total": 1
}
```

### `POST /api/admin/roles`

创建角色。内置角色不可重复创建。

### `GET /api/admin/roles/{roleId}`

返回详情抽屉：

- 基础信息。
- 成员。
- 系统权限。
- 租户范围。
- 子系统身份默认值。
- 审计记录。

### `PATCH /api/admin/roles/{roleId}`

编辑名称、描述、角色类型。内置角色的编码和类型不可修改。

### `PUT /api/admin/roles/{roleId}/members`

维护角色成员。成员可指定租户范围。

### `PUT /api/admin/roles/{roleId}/permissions`

复用权限配置域的矩阵保存能力，写入 `portal_permission_assignments.subject_type='role'`。

### `DELETE /api/admin/roles/{roleId}`

删除角色前必须检查成员和授权：

- 有成员时禁止删除，要求先迁移或移除成员。
- 内置角色禁止删除。
- 删除成功写入审计。

## 内置角色

初始角色：

- `super-admin`：门户超级管理员。
- `user`：门户普通用户。
- `subsystem-admin`：子系统管理员基础身份。
- `audit-viewer`：审计查看员，可选。
- `system-integrator`：系统接入管理员，可选。

## 验收

- 角色列表能展示成员数量、绑定系统数量和更新时间。
- 角色权限能作为用户最终权限来源之一被解释。
- 删除角色时能提示受影响用户，不能误删内置角色。
