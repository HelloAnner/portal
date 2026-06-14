# 企业门户系统

企业内部各类业务系统的统一入口，同时承担统一身份、租户、角色、权限和子系统接入配置的控制面职责。

后端采用 Rust（Axum + SQLx）实现，作为唯一可信控制面；前端为 Next.js 应用，仅消费后端 API。

## 主要功能

- 统一登录与认证
- 门户首页（系统入口聚合、租户切换）
- 我的资料与我的权限
- 管理后台：概览、用户管理、租户管理、角色管理、系统目录、权限配置、子系统管理员、子系统接入、安全审计
- 子系统一次性授权码跳转

## 技术栈

- **后端**：Rust + Axum + Tokio + SQLx + PostgreSQL
- **前端**：Next.js 16 App Router + TypeScript
- **样式**：Tailwind CSS 4
- **部署**：Rust 服务直接托管前端静态产物，单环境单 `.env` 部署

## 项目结构

```text
apps/api-rs/        # Rust 后端服务
apps/web/           # Next.js 前端工程
crates/web_embed/   # 将前端静态产物内嵌到 Rust 二进制
docs/               # 产品、设计、技术和功能域文档
```

## 快速开始

```bash
# 启动 PostgreSQL
docker run -d --name portal-postgres -e POSTGRES_USER=portal -e POSTGRES_PASSWORD=portal -e POSTGRES_DB=portal -p 5432:5432 postgres:16-alpine

# 安装依赖
pnpm install

# 构建 Rust 后端
cargo build --release --bin portal-api

# 初始化数据
pnpm db:seed

# 方式一：分别启动后端和前端（开发）
pnpm dev:api      # Rust 后端，默认 http://localhost:8080
pnpm dev          # Next.js 前端，默认 http://localhost:3000

# 方式二：直接运行生产二进制（前端静态产物需先构建）
cd apps/web && pnpm build
./target/release/portal-api
```

默认管理员账号 `admin` / `admin123`。

生产部署时，将 `apps/web/out` 构建产物与 `target/release/portal-api` 二进制发布到同一环境，配置同一份 `.env` 后启动即可。
