# 管理后台概览后台实现

## 对应页面

- `Admin Overview`
- 指标卡片
- 待处理事项
- 最近审计记录

## 后台职责

概览页提供只读运营状态，不承载危险写操作。它聚合用户、系统、接入、权限和审计数据，用于让管理员快速发现风险和进入对应详情页。

## 接口

### `GET /api/admin/overview`

返回：

```json
{
  "stats": {
    "userTotal": 128,
    "activeSystemTotal": 6,
    "portalManagedSystemTotal": 4,
    "subsystemEntry24h": 342,
    "highRiskPermissionChanges24h": 3
  },
  "todos": [
    {
      "type": "integration_incomplete",
      "title": "DocuMind 接入配置未完成",
      "targetType": "system",
      "targetId": "..."
    }
  ],
  "recentAudits": []
}
```

## 指标口径

- 用户总数：`portal_users` 中非 `archived` 用户。
- 启用系统数：`portal_systems.status='active'`。
- 门户托管认证系统数：`portal_systems.portal_managed=true and auth_enabled=true`。
- 最近 24 小时进入子系统次数：`portal_audit_events.action='subsystem.enter'`。
- 高风险权限变更次数：撤销管理员、授予超级管理员、停用租户、停用系统等事件。

## 待处理事项

待处理事项通过查询生成，不单独建表：

- 接入中系统缺少 `callback_url`、公钥或校验端点。
- 子管理员授权即将过期。
- 最近 24 小时凭证校验失败超过阈值。
- 存在状态为 `pending` 且已分配权限的用户。
- 租户停用但仍有活跃授权。

## 权限

只有具备门户超级管理员、审计查看员或配置管理员角色的用户可访问。不同角色可看到不同内容：

- 超级管理员：全部指标。
- 审计查看员：审计和风险指标。
- 系统接入管理员：系统和接入待办。

## 验收

- 点击指标能带筛选条件跳转到对应后台列表。
- 待处理事项中的目标 ID 能打开系统、用户、租户或权限详情。
- 概览页不直接执行禁用、撤销等高风险操作。
