# 项目定位

本项目是「门户」系统，定位为企业内部各种系统的统一入口。

门户系统主要承担以下职责：

- 作为企业内部系统的统一访问入口，承载跨系统导航、入口聚合与用户访问路径。
- 负责用户登录相关能力，包括登录流程、身份识别、会话与权限入口等。
- 负责用户管理相关能力，包括用户信息维护、账号管理、组织或角色相关管理逻辑等。

# 关联项目

门户软件还包含以下关联子系统。必要时，可以查看并修改这些项目的源代码来确认或调整整体逻辑。

- `$HOME/Northline`：企业数据库问数系统的源代码，是门户软件的一部分。
- `$HOME/Documind`：企业文档 RAG 系统的源代码，是门户软件的一部分。本机当前目录名可能为 `$HOME/DocuMind`，注意大小写差异。

# 协作说明

在处理本项目问题时，优先理解门户系统作为统一入口的职责边界。若问题涉及数据库问数、文档 RAG、登录态传递、用户体系联动或跨系统跳转，可以同时查看 `Northline` 和 `Documind` 相关代码，确认端到端逻辑后再修改。

# 运行环境

- `ssh northline` 是服务器环境，不是普通跳板机；部署、端口、日志、PostgreSQL / Redis / RabbitMQ 等运行时状态都以这台机器为准。
- 默认优先服务器环境：部署、排查、验收和端到端测试都先在 `ssh northline` 上确认，不默认使用本地环境复现。
- 本地仓库主要用于开发、阅读代码和构建产物；除非用户明确要求，本地不作为默认部署或测试环境。
- 门户、Northline、DocuMind 这三个系统都以服务器当前运行状态为准；排查问题时先确认服务器上的进程、端口、日志和数据。
- 服务器部署采用 `releases/<timestamp>` + `current` + `shared` 的发布结构。
- 门户服务器部署根目录是 `/opt/portal`，本地端口 `7777`，对外通过 Nginx `:6688/` 和 `:6688/portal/` 访问。
- 本地 Northline 源代码位于 `$HOME/Northline`，必要时可以查看其 `Makefile`、部署脚本和 `AGENTS.md` 作为同机部署参考。
- DocuMind 服务端口固定为服务器本地 `5555`，对外通过原生 Nginx 的 `:6688/documind/` 访问。
- Northline 同机运行，Nginx 的 `:6688/northline/` 代理到服务器本地 Northline 端口 `6666`。
- 当前门户联调以门户登录为统一入口，用户登录门户后通过门户下发的一次性 ticket 进入 Northline 或 DocuMind。
- 服务器当前联调超级管理员账号用于验收模拟：`admin` / `adminadmin`。

# 本地开发

门户后端已迁移为 Rust（Axum + SQLx）实现，前端为 Next.js 应用，通过 `/api/*` 调用 Rust 后端。

- 本地只作为开发和构建环境，不作为默认排查、部署或验收环境。
- 本地检查优先使用 `cargo check -p portal-api` 和 `pnpm --filter @portal/web build`。
- Fresh DB 不再默认创建管理员；首次访问门户会进入 `/setup` 配置超级管理员。

生产部署时，先构建前端静态产物（`apps/web/out`），再由 `portal-api` 二进制直接托管静态资源并提供 API。


<claude-mem-context>
# Memory Context

# claude-mem status

This project has no memory yet. The current session will seed it; subsequent sessions will receive auto-injected context for relevant past work.

Memory injection starts on your second session in a project.

`/learn-codebase` is available if the user wants to front-load the entire repo into memory in a single pass (~5 minutes on a typical repo, optional). Otherwise memory builds passively as work happens.

Live activity: http://localhost:37701
How it works: `/how-it-works`

This message disappears once the first observation lands.
</claude-mem-context>
