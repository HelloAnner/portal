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

# 本地开发

门户后端已迁移为 Rust（Axum + SQLx）实现，前端为 Next.js 应用，通过 `/api/*` 调用 Rust 后端。

1. 确保 PostgreSQL 在本地运行（默认端口 5432，数据库/用户/密码均为 portal）。可用 Docker：
   ```bash
   docker run -d --name portal-postgres -e POSTGRES_USER=portal -e POSTGRES_PASSWORD=portal -e POSTGRES_DB=portal -p 5432:5432 postgres:16-alpine
   ```
2. 安装依赖：`pnpm install`
3. 构建 Rust 后端：`cargo build --release --bin portal-api`
4. 初始化数据：`pnpm db:seed`（对应 `cargo run --bin portal-seed`）
5. 启动 Rust API：`pnpm dev:api`（默认端口 8080）
6. 启动前端开发服务器：`pnpm dev`（默认端口 3000，通过 `NEXT_PUBLIC_API_URL` 调用 Rust 后端）
7. 默认管理员账号：`admin` / `admin123`

生产部署时，先构建前端静态产物（`apps/web/out`），再由 `portal-api` 二进制直接托管静态资源并提供 API。

<!-- BEGIN:nextjs-agent-rules -->
# This is NOT the Next.js you know

This version has breaking changes — APIs, conventions, and file structure may all differ from your training data. Read the relevant guide in `node_modules/next/dist/docs/` before writing any code. Heed deprecation notices.
<!-- END:nextjs-agent-rules -->
