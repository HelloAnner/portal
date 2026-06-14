# 子系统接入后台实现

## 对应页面

- `Subsystem Integration`
- `Northline 接入配置` 抽屉
- 接入检查按钮
- 子系统 `.env` 示例代码块

## 后台职责

接入域维护门户与子系统之间的认证协作配置，包括托管认证开关语义、发行方、公钥或校验端点、回调地址、凭证有效期和接入检查结果。

## 数据表

- `portal_systems`
- `portal_integration_configs`
- `portal_audit_events`

## 接口

### `GET /api/admin/integrations`

返回接入系统列表：

```json
{
  "items": [
    {
      "systemCode": "northline",
      "status": "active",
      "authMode": "authorization_code",
      "issuer": "https://portal.example.com",
      "callbackUrl": "https://northline.example.com/auth/portal/callback",
      "verifyEndpoint": "https://northline.example.com/auth/portal/check",
      "lastCheckAt": "...",
      "lastCheckResult": {
        "success": true
      }
    }
  ]
}
```

### `GET /api/admin/integrations/{systemId}`

返回抽屉详情：

- `PORTAL_MANAGED` 语义。
- `PORTAL_AUTH_ENABLED` 语义。
- `PORTAL_ISSUER`。
- 凭证有效期。
- 身份上下文字段。
- 权限声明字段。
- `.env` 示例。

### `PUT /api/admin/integrations/{systemId}`

保存接入配置。系统必须先存在于系统目录。

### `POST /api/admin/integrations/{systemId}/check`

接入检查：

- 系统编码是否匹配。
- 回调地址是否可访问。
- 子系统是否声明门户托管认证。
- 凭证校验方式是否可用。
- 子系统是否仍暴露生产本地登录入口。

检查结果写入 `portal_integration_configs.last_check_result`。

## 子系统 `.env` 示例生成

按系统生成：

```env
SYSTEM_CODE=northline
PORTAL_MANAGED=true
PORTAL_AUTH_ENABLED=true
PORTAL_ISSUER=https://portal.example.com
PORTAL_AUTH_MODE=authorization_code
PORTAL_AUTH_CALLBACK=/auth/portal/callback
PORTAL_PUBLIC_KEY_PATH=/app/config/portal-public-key.pem
```

字段名可适配 Northline、DocuMind 现有技术栈，但必须保留语义。

## 验收

- 管理员能看到每个系统的接入状态和最近校验结果。
- 切换门户托管认证前，后台能提示需要关闭或旁路子系统本地登录。
- 接入检查失败不会影响已发布系统，但不能标记为接入完成。
