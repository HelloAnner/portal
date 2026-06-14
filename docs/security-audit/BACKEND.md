# 安全与审计后台实现

## 对应页面

- `Security Audit`
- 审计详情抽屉
- 复制筛选和导出

## 后台职责

安全与审计域负责记录关键事件、支持筛选查询、导出审计结果，并为高风险行为提供可回溯证据。它不替代子系统业务审计，但必须覆盖门户登录、授权、凭证、接入和系统入口变更。

## 数据表

- `portal_audit_events`
- 关联读取 `portal_users`、`portal_systems`、`portal_tenants`

## 写入事件

必须写入：

- `login.success`
- `login.failure`
- `logout.success`
- `subsystem.enter`
- `subsystem.ticket.issue`
- `subsystem.ticket.exchange.failure`
- `user.create`
- `user.update`
- `user.disable`
- `permission.change`
- `sub_admin.change`
- `system.change`
- `integration.change`
- `integration.check`
- `tenant.change`
- `role.change`

## 接口

### `GET /api/admin/audits`

筛选：

- `startAt`
- `endAt`
- `actorUserId`
- `action`
- `systemId`
- `tenantId`
- `result`
- `keyword`

返回审计表格：

```json
{
  "items": [
    {
      "id": "...",
      "occurredAt": "...",
      "actorName": "张三",
      "action": "permission.change",
      "targetType": "permission",
      "targetId": "...",
      "result": "success",
      "ipAddress": "10.0.0.1",
      "userAgent": "Mozilla/5.0"
    }
  ],
  "total": 1
}
```

### `GET /api/admin/audits/{auditId}`

返回详情：

- 操作前数据。
- 操作后数据。
- 失败原因。
- 关联请求 ID。
- 操作人、系统、租户。

### `POST /api/admin/audits/export`

按当前筛选条件创建导出任务。导出内容可为 CSV 或 JSONL，必须记录导出审计。

## 安全规则

- 审计写入失败不能静默吞掉。关键写操作应在同一事务中写审计，或写入可靠队列。
- `before_data` 和 `after_data` 需要脱敏密码、密钥、令牌。
- 审计查询默认按时间倒序，最长范围可配置。
- 普通管理员只能查看自己有权限管理范围内的审计；超级管理员和审计查看员可看全部。

## 验收

- 登录、授权、子系统跳转、凭证失败和接入变更都有记录。
- 审计详情能展示 JSON 差异和失败原因。
- 导出不会泄露密码、私钥、session 或一次性授权码明文。
