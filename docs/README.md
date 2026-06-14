# 门户系统文档索引

## 产品与设计

- [PRD](./prd.md)
- [UI 设计说明](./ui.md)
- [技术方案](./tech.md)
- [后台实现总览](./backend-implementation.md)

## 页面与功能域

| 域 | 功能说明 | 后台实现 |
| --- | --- | --- |
| 登录与认证 | [authentication](./authentication/README.md) | [BACKEND](./authentication/BACKEND.md) |
| 门户首页 | [portal-home](./portal-home/README.md) | [portal-home](./portal-home/README.md) |
| 我的资料与我的权限 | [profile-permissions](./profile-permissions/README.md) | [profile-permissions](./profile-permissions/README.md) |
| 管理后台概览 | [admin-overview](./admin-overview/README.md) | [admin-overview](./admin-overview/README.md) |
| 用户管理 | [user-role-management](./user-role-management/README.md) | [BACKEND](./user-role-management/BACKEND.md) |
| 租户管理 | [tenant-management](./tenant-management/README.md) | [tenant-management](./tenant-management/README.md) |
| 角色管理 | [role-management](./role-management/README.md) | [role-management](./role-management/README.md) |
| 权限配置 | [permission-configuration](./permission-configuration/README.md) | [BACKEND](./permission-configuration/BACKEND.md) |
| 系统目录 | [system-catalog](./system-catalog/README.md) | [BACKEND](./system-catalog/BACKEND.md) |
| 子系统管理员 | [sub-admin](./sub-admin/README.md) | [BACKEND](./sub-admin/BACKEND.md) |
| 子系统接入 | [subsystem-integration](./subsystem-integration/README.md) | [BACKEND](./subsystem-integration/BACKEND.md) |
| 安全与审计 | [security-audit](./security-audit/README.md) | [BACKEND](./security-audit/BACKEND.md) |
| 无权限页 | [no-permission](./no-permission/README.md) | [no-permission](./no-permission/README.md) |

## 实现基线

- 门户后端采用 Rust，和 Northline、DocuMind 的后端实现逻辑保持一致。
- 门户采用单环境部署，通过同一份 `.env` 启动。
- 默认启动必须连接 PostgreSQL。
- 支持通过 `PG_HOST`、`PG_PORT`、`PG_DATABASE`、`PG_USER`、`PG_PASSWORD`、`PG_SCHEMA` 配置数据库位置和 schema。
- PostgreSQL 保存用户、租户、角色、系统目录、权限、子管理员、接入配置、会话、子系统授权码和审计。
- 前端部分正常使用，通过 API 消费 Rust 后端提供的可信身份、权限和管理能力。
- Northline、DocuMind 等子系统在门户托管模式下消费门户签发或换取的身份上下文。
