# 门户系统设计规范

## Overview

门户系统采用 Apple 官网式的极简产品展示风格。界面由全出血的 "tile" 区块堆叠而成：纯白、羊皮纸、近黑三种表面交替，形成清晰的视觉节奏。所有交互元素统一使用单一 Action Blue，没有第二品牌色。UI 极度克制——无装饰性渐变、无卡片阴影、无多余边框，让内容本身成为焦点。

**核心特征：**

- 摄影/产品优先的展示方式，UI 退后成为画框。
- 全出血 tile 交替：纯白 ↔ 羊皮纸 ↔ 近黑，颜色变化即分隔。
- 单一蓝色强调色 `#0066cc` 承载所有交互：链接、主 CTA、焦点环。
- 两种按钮语法：蓝色胶囊 CTA（`{rounded.pill}`）与紧凑工具矩形（`{rounded.sm}`）。
- 字体使用 SF Pro Display（标题）+ SF Pro Text（正文），在 Pencil 中以 `system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` 回退；必要时使用 Inter 作为跨平台替代。
- 标题 600 字重 + 负字距，正文 17px / 400 / 1.47，刻意不使用 500 字重。
- 仅有一种阴影：产品渲染图使用 `rgba(0, 0, 0, 0.22) 3px 5px 30px`；卡片、按钮、文字均无阴影。

## 单页面架构

门户收敛为单一页面「Portal」，所有入口通过左侧边栏访问：

- **左侧边栏（固定 260px，背景 `canvas` 纯白）**
  - 顶部：品牌 Logo + "Portal" 字样。
  - **普通用户菜单**：首页、全部系统、Northline、DocuMind、我的资料、我的权限。
  - **管理员菜单**（仅管理员可见）：管理后台、概览、用户管理、租户管理、系统目录、权限配置、角色管理、子系统管理员、子系统接入、安全审计。
  - 底部：用户头像 + 名称 + 角色徽章，以及反馈/退出入口。
- **右侧内容区**
  - 顶部栏：当前页面标题、搜索胶囊、租户切换、通知与头像。
  - 主内容区：欢迎 Hero tile + 系统入口 utility cards 网格 + 可选深色产品展示 tile。

侧边栏通过两个可复用组件实例化：`Sidebar/User` 与 `Sidebar/Admin`。

## 色彩系统

### 品牌与强调色

| Token | Value | 用途 |
|-------|-------|------|
| `primary` | `#0066cc` | 所有交互元素：主 CTA、文字链接、焦点环 |
| `primary-focus` | `#0071e3` | 按钮键盘焦点环 |
| `primary-on-dark` | `#2997ff` | 暗色表面上的文字链接 |

### 表面色

| Token | Value | 用途 |
|-------|-------|------|
| `canvas` | `#ffffff` | 主画布、内容区、白色卡片 |
| `canvas-parchment` | `#f5f5f7` | 羊皮纸交替 tile、页脚、主内容区背景 |
| `surface-pearl` | `#fafafc` | 次级幽灵按钮填充 |
| `surface-tile-1` | `#272729` | 主暗色 tile |
| `surface-tile-2` | `#2a2a2c` | 微亮的相邻暗色 tile |
| `surface-tile-3` | `#252527` | 微暗的底部 tile / 视频框 |
| `surface-black` | `#000000` | 全局导航栏背景 |
| `surface-chip-translucent` | `#d2d2d7` | 悬浮在图片上的圆形控制按钮底色（64% 透明度） |

### 文字色

| Token | Value | 用途 |
|-------|-------|------|
| `ink` | `#1d1d1f` | 标题、正文、深色按钮填充 |
| `body` | `#1d1d1f` | 浅色表面上的正文（与 ink 同值） |
| `body-on-dark` | `#ffffff` | 暗色 tile 与导航栏上的文字 |
| `body-muted` | `#cccccc` | 暗色 tile 上的次级文字 |
| `ink-muted-80` | `#333333` | 珍珠按钮上的文字 |
| `ink-muted-48` | `#7a7a7a` | 禁用文字、法律小字 |

### 分隔线与边框

| Token | Value | 用途 |
|-------|-------|------|
| `divider-soft` | `#f0f0f0` | 次级按钮的柔和边框 |
| `hairline` | `#e0e0e0` | 卡片 1px 边框 |

### 语义色

| Token | Value | 用途 |
|-------|-------|------|
| `color-success` | `#34c759` | 成功、启用状态 |
| `color-warning` | `#ff9500` | 维护中、警告 |
| `color-error` | `#ff3b30` | 错误、禁用、异常 |
| `color-info` | `#2997ff` | 信息提示（暗面链接同色） |

## 字体与排版

### 字体家族

- **Display**：`SF Pro Display, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
- **Body / UI**：`SF Pro Text, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
- **替代方案**：跨平台无法使用 SF Pro 时，使用 Inter；标题需略微收紧 `letter-spacing`（约 `-0.01em`）以复现 Apple 紧致感。

### 层级

| Token | Size | Weight | Line Height | Letter Spacing | Use |
|---|---|---|---|---|---|
| `typography.hero-display` | 56px | 600 | 1.07 | -0.28px | Hero 大标题 |
| `typography.display-lg` | 40px | 600 | 1.10 | 0 | 区块大标题 |
| `typography.display-md` | 34px | 600 | 1.47 | -0.374px | 中型标题 |
| `typography.lead` | 28px | 400 | 1.14 | 0.196px | 产品 tile 副标题 |
| `typography.tagline` | 21px | 600 | 1.19 | 0.231px | 子导航分类名、副标题 |
| `typography.body-strong` | 17px | 600 | 1.24 | -0.374px | 强调正文 |
| `typography.body` | 17px | 400 | 1.47 | -0.374px | 默认正文 |
| `typography.caption` | 14px | 400 | 1.43 | -0.224px | 次级说明、按钮文字 |
| `typography.caption-strong` | 14px | 600 | 1.29 | -0.224px | 强调说明 |
| `typography.button-utility` | 14px | 400 | 1.29 | -0.224px | 工具/导航按钮 |
| `typography.fine-print` | 12px | 400 | 1.0 | -0.12px | 小字、法律声明 |
| `typography.nav-link` | 12px | 400 | 1.0 | -0.12px | 全局导航链接 |

### 原则

- 标题使用 600 字重，不使用 700；正文使用 400，不使用 500。
- 17px 及以上的字号使用负字距，营造 Apple 紧致排版。
- 正文固定 17px / 1.47，这是 Apple 的阅读节奏。

## 布局

### 间距系统

| Token | Value |
|---|---|
| `spacing.xxs` | 4px |
| `spacing.xs` | 8px |
| `spacing.sm` | 12px |
| `spacing.md` | 17px |
| `spacing.lg` | 24px |
| `spacing.xl` | 32px |
| `spacing.xxl` | 48px |
| `spacing.section` | 80px |

- 区块内边距：`spacing.section`（80px）。
- 卡片内边距：`spacing.lg`（24px）。
- 按钮内边距：胶囊按钮 11px × 22px；工具按钮 8px × 15px。

### 网格与容器

- 左侧边栏固定 260px。
- 右侧内容区最大宽度随屏幕变化，通常 980px–1440px。
- 系统入口卡片网格：桌面 3–4 列，平板 2 列，手机 1 列。

### 留白哲学

区块顶部至少预留 64px 空气，产品/卡片图像与最近内容保持 ≥40px 距离。侧边栏与内容区之间无多余分隔，依靠表面色差异区分。

## 层次与深度

| Level | Treatment | Use |
|---|---|---|
| Flat | 无阴影、无边框 | 全出血 tile、导航、页脚 |
| Soft hairline | 1px `rgba(0,0,0,0.08)` 边框 | 工具卡片、次级导航分隔 |
| Backdrop blur | 磨砂玻璃效果 | 粘性子导航、底部浮动条 |
| Product shadow | `rgba(0,0,0,0.22) 3px 5px 30px` | 仅用于产品渲染图 |

**阴影原则**：整个系统只有产品渲染图使用阴影；UI 元素通过表面色变化获得层次。

## 形状

| Token | Value | Use |
|---|---|---|
| `rounded.none` | 0px | 全出血 tile |
| `rounded.sm` | 8px | 深色工具按钮、内联卡片图像 |
| `rounded.md` | 11px | 珍珠胶囊按钮 |
| `rounded.lg` | 18px | 系统入口 utility cards |
| `rounded.pill` | 9999px | 主 CTA、搜索框、配置器选项 chip |
| `rounded.full` | 9999px / 50% | 圆形控制按钮、头像 |

## 核心组件

### 顶部栏

**`top-bar`** — 高度 52px，背景 `canvas-parchment` 或 `canvas`。左侧为当前页面标题（`display-md` 或 `tagline`），右侧为搜索胶囊、租户切换、通知图标、用户头像。

### 侧边栏

**`sidebar-base`** — 宽度 260px，高度填满视口，背景 `canvas`（纯白）。顶部品牌区，中部按角色分组导航，底部用户区。

- **`sidebar-user`**：普通用户入口分组。
- **`sidebar-admin`**：管理员入口分组，在普通用户分组基础上增加管理后台与安全审计组。

### 导航项

**`nav-item`** — 横向布局，图标 + 标签，`rounded.sm`（8px）圆角，内边距 10px × 12px。

- 默认：透明背景，`ink-muted-80` 图标与文字。
- 悬停：`canvas-parchment` 背景（管理员/用户侧边栏底色本身已是 parchment，悬停使用 `canvas` 形成轻微对比）。
- 选中：`canvas` 背景 + `primary` 图标与文字。

### 按钮

**`button-primary`** — 背景 `primary`（#0066cc），文字 `canvas`（白色），`rounded.pill`，内边距 11px × 22px，字号 17px / 400。

**`button-secondary-pill`** — 透明背景，`primary` 文字 + `primary` 1px 边框，`rounded.pill`。

**`button-dark-utility`** — 背景 `ink`，文字 `body-on-dark`，`rounded.sm`（8px），内边距 8px × 15px。

**`button-pearl-capsule`** — 背景 `surface-pearl`，文字 `ink-muted-80`，3px `divider-soft` 边框，`rounded.md`（11px）。

### 卡片

**`hero-tile`** — 全出血 tile，背景 `canvas` 或 `canvas-parchment`，内边距 80px。居中堆叠：标题（`hero-display`）→ 副标题（`lead`）→ 双 CTA（`button-primary` + `button-secondary-pill`）→ 产品/系统图示。

**`system-card`** — 系统入口卡片。背景 `canvas`，1px `hairline` 边框，`rounded.lg`（18px），内边距 24px。顶部图标区，下方系统名称（`body-strong`）、说明（`body`）、"进入" 链接或按钮。

**`dark-tile`** — 全出血暗色 tile，背景 `surface-tile-1`，文字 `body-on-dark`。用于管理员概览、产品展示或数据面板。

### 输入框

**`search-input`** — 背景 `canvas`，文字 `ink`，1px `rgba(0,0,0,0.08)` 边框，`rounded.pill`，内边距 12px × 20px，高度 44px。左侧搜索图标。

### 徽章与状态

**`badge-pill`** — 背景 `canvas-parchment`，文字 `ink-muted-80`，`rounded.pill`，内边距 4px × 12px，字号 `caption`。

**`status-dot`** — 8px 圆点 + 文字说明。成功绿色 `#22C55E`、警告琥珀 `#F59E0B`、错误红色 `#EF4444` 保留作为语义色。

## Do's and Don'ts

### Do

- 使用 `primary`（#0066cc）作为唯一交互色。
- 标题使用 600 字重 + 负字距。
- 正文使用 17px / 400 / 1.47。
- 交替使用 `canvas`、`canvas-parchment`、`surface-tile-1` 创造区块节奏。
- 将产品/系统图示作为区块核心，UI 元素退后。
- 仅对产品渲染图使用系统阴影。

### Don't

- 引入第二个强调色。
- 给卡片、按钮、文字添加阴影。
- 使用 500 字重。
- 使用渐变作为装饰背景。
- 给全出血 tile 加圆角。
- 在正文使用小于 1.47 的行高。

## 响应式

### 断点

| 名称 | 宽度 | 关键变化 |
|---|---|---|
| 手机 | < 640px | 侧边栏收起为汉堡菜单；卡片 1 列；Hero 标题降至 28–34px |
| 平板 | 640–1023px | 卡片 2 列；侧边栏可折叠 |
| 桌面 | 1024–1440px | 完整侧边栏；卡片 3 列 |
| 宽屏 | > 1440px | 内容锁定在 1440px，边距吸收额外宽度 |

### 触摸目标

- 主按钮最小 44px 高度。
- 图标按钮 44 × 44px。
- 导航项整行可点击。

## 与现有文档的关系

- 本规范覆盖视觉风格与组件表现。
- 信息架构、页面职责、交互流程仍以 `docs/ui.md` 为准。
- 后端 API 与数据模型以 `docs/portal-home/README.md` 等实现文档为准。
