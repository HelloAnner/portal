# 无权限页后台实现

## 对应页面

- `No Permission`
- 无系统权限、无后台权限、凭证过期、凭证无效等恢复路径

## 后台职责

无权限页不是单独业务域，但需要统一错误原因和恢复动作。后台应返回机器可读错误码和可展示的目标信息，让前端根据原因展示返回首页、申请权限、重新登录等按钮。

## 接口

### `GET /api/access-denied/context`

查询参数：

- `reason`
- `systemCode`
- `tenantId`
- `returnTo`

返回：

```json
{
  "reason": "PERMISSION_DENIED",
  "title": "暂无权限访问",
  "targetName": "Northline",
  "actions": {
    "backHome": true,
    "requestPermission": true,
    "relogin": false
  },
  "safeReturnTo": "/"
}
```

## 错误来源

- `AUTH_REQUIRED`：未登录或 session 过期，展示重新登录。
- `PERMISSION_DENIED`：没有系统访问权限，展示返回首页和申请权限。
- `SYSTEM_DISABLED`：系统停用，展示返回首页。
- `SYSTEM_MAINTENANCE`：系统维护中，展示返回首页。
- `TENANT_DISABLED`：租户停用，展示返回首页。
- `INVALID_SUBSYSTEM_TICKET`：授权码无效或过期，展示重新登录或重新进入。

## 权限申请

若 `ALLOW_PERMISSION_REQUEST=true`，无权限页可以调用：

### `POST /api/permission-requests`

请求：

```json
{
  "systemCode": "documind",
  "tenantId": "...",
  "reason": "需要访问知识库"
}
```

首版可以只写入审计或通知管理员；后续可扩展正式审批流。

## 验收

- 所有后端拒绝访问的场景都有稳定错误码。
- 无权限页不会暴露敏感权限配置。
- 回跳地址经过白名单校验，不产生开放重定向。
