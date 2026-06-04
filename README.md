<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="Glean" />
</p>

<h1 align="center">Glean</h1>

<p align="center">
  <a href="./README.md">中文</a> · <a href="./README_EN.md">English</a>
</p>

<p align="center">
  本地优先、AI 原生的 macOS 文件管理器<br/>
  A local-first, AI-native macOS file manager
</p>

<p align="center">
  从你电脑里散落的文件中拾取价值 —— 找得到，记得住，自动归档。
</p>

<p align="center">
  <img src="https://img.shields.io/badge/status-WIP-orange" alt="status" />
  <img src="https://img.shields.io/badge/platform-macOS%2013%2B-blue" alt="platform" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="license" />
  <img src="https://img.shields.io/badge/Tauri-v2-blueviolet" alt="tauri" />
  <img src="https://img.shields.io/badge/Vue-3.5-brightgreen" alt="vue" />
</p>

---

## ✨ 项目定位 / What is Glean

Glean 解决的是 **"我电脑里有 5000 个文件,我知道我看过,但我找不到"** 的痛点。

**核心能力**:

- 🗂 **智能索引** — 扫描 `~/Documents`、`~/Downloads`、`~/Desktop`,理解 PDF / Markdown / 图片 / 邮件
- 🔍 **语义检索** — 自然语言查询:"上周那个讲融资的 PDF 在哪"
- 🤖 **Agent 执行** — 找文件 + 整理 + 重命名 + 自动归档
- 🔒 **本地优先** — 所有数据本地存储,BYOK 调用 LLM,零服务器依赖

---

## 📸 预览 / Preview

<p align="center">
  <img src="./screenshots/hero.png" width="800" alt="Glean 主界面" />
</p>

<p align="center">
  <img src="./screenshots/demo.gif" width="800" alt="Glean 演示" />
</p>

<details>
<summary>📸 更多截图</summary>

| 搜索面板 | AI 助手 + 工具调用 |
|:---:|:---:|
| <img src="./screenshots/search.png" width="400" alt="搜索" /> | <img src="./screenshots/chat-agent.png" width="400" alt="AI 助手" /> |

| 标签管理 | 设置页 |
|:---:|:---:|
| <img src="./screenshots/tags.png" width="400" alt="标签" /> | <img src="./screenshots/settings.png" width="400" alt="设置" /> |

</details>

---

## 🛠 技术栈 / Tech Stack

| 层 | 技术 |
|----|------|
| 桌面壳 | Tauri v2 |
| 前端 | Vue 3.5 + TypeScript + Vite 6 |
| UI | Tailwind CSS v4 + Lucide Icons |
| 状态 | Pinia 3 + VueUse |
| 后端 | Rust |
| 数据库 | SQLite (rusqlite + rusqlite_migration) + FTS5 |
| 文件监听 | notify |
| 向量库 | LanceDB (规划中) |
| 本地 LLM | MLX (规划中) |
| 本地 Embedding | Qwen3-Embedding / bge-m3 (规划中) |
| OCR | macOS Vision Framework (规划中) |

---

## 📦 安装与运行 / Getting Started

### 前置依赖

- macOS 13+
- Node.js 20+
- pnpm 11+
- Rust 1.75+ (`rustup`)
- Xcode Command Line Tools

### 开发模式

```bash
pnpm install
pnpm tauri dev
```

应用启动后,默认窗口 1280×800,最小尺寸 900×600。

### 生产构建

```bash
pnpm tauri build
```

产物位于 `src-tauri/target/release/bundle/`。

### 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| `⌘ + Shift + Space` | 显示 / 隐藏主窗口 |
| `⌘ + K` | 文件搜索面板 |
| `⌘ + B` | 切换 AI 助手面板 |
| `Enter` | 打开选中文件 / 发送消息 |
| `Esc` | 关闭面板/弹窗 |

---

## 📥 下载与安装 / Download & Install

### macOS（Homebrew Cask 即将上线）

1. 前往 [Releases](https://github.com/WalkAlone0325/glean/releases) 下载最新 `Glean_<version>_aarch64.dmg`（Apple Silicon）或 `x86_64.dmg`（Intel）
2. 打开 `.dmg`，将 Glean 拖入 Applications
3. 首次启动若提示"无法验证开发者"，在终端运行：
   ```bash
   xattr -dr com.apple.quarantine /Applications/Glean.app
   ```

### 从源码构建

```bash
git clone https://github.com/WalkAlone0325/glean.git
cd glean
pnpm install
pnpm tauri build
```

---

## 🔐 隐私 / Privacy

- 所有文件索引、对话历史、配置均存储在本地 SQLite，不上传任何服务器
- API Key 仅在你使用 AI 助手时用于调用你配置的 LLM Provider
- 不收集任何遥测、崩溃报告或使用统计

[完整隐私声明 →](./docs/PRIVACY.md)

---

## 🗺 路线图 / Roadmap

详见 [notes/plan.md](./notes/plan.md)（开发计划，仅本地可见）。

| 阶段 | 状态 |
|------|------|
| Phase 0：项目地基 | ✅ |
| Phase 1：文件索引 | ✅ |
| Phase 2：语义检索 | ✅ |
| Phase 3：Agent 工具调用 | ✅ |
| Phase 4：打磨 + 分发 | 🚧 |

---

## 📂 项目结构 / Project Structure

```
glean/
├── src/                          # Vue 前端
│   ├── components/               #   UI 组件 (SearchPalette ...)
│   ├── stores/                   #   Pinia stores (app, search ...)
│   ├── App.vue
│   └── main.ts
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── commands.rs           #     Tauri command 入口
│   │   ├── db/                   #     SQLite + 迁移
│   │   ├── scanner/              #     文件扫描器
│   │   │   ├── walker.rs         #       目录遍历
│   │   │   ├── scheduler.rs      #       调度器
│   │   │   ├── watcher.rs        #       文件监听
│   │   │   └── text.rs           #       文本提取
│   │   ├── search/               #     全文搜索
│   │   └── lib.rs
│   ├── icons/                    #     应用图标 (macOS / Windows)
│   ├── capabilities/             #     Tauri 权限配置
│   └── tauri.conf.json
├── design/                       # 图标设计稿 (SVG)
├── .github/workflows/            # CI (PR 检查) + Release (自动打包)
└── package.json
```

---

## 🗺 路线图 / Roadmap

| 阶段 | 时间 | 状态 | 关键交付 |
|------|------|------|----------|
| Phase 0:项目地基 | 第 1 周 | ✅ 完成 | Tauri 骨架、CI/CD、图标 |
| Phase 1:文件索引核心 | 第 2-4 周 | 🚧 进行中 | Walker、Scheduler、Watcher、SQLite FTS5 |
| Phase 2:语义检索 | 第 5-7 周 | ⏳ 待开始 | Embedding、LanceDB、向量混合检索 |
| Phase 3:Agent 能力 | 第 8-10 周 | ⏳ 待开始 | LLM 调用、工具调用、自动归档 |
| Phase 4:打磨 + 分发 | 第 11-12 周 | ⏳ 待开始 | 性能优化、签名、公测 |

---

## 🧪 开发指南 / Development

### 代码规范

- **前端**:ESLint + Prettier (`pnpm lint` / `pnpm format`)
- **后端**:`cargo fmt` / `cargo clippy`
- **类型检查**:`pnpm build` 会先跑 `vue-tsc --noEmit`

### 数据库迁移

新增迁移:在 `src-tauri/src/db/migrations.rs` 中追加 `M` 项,递增版本号。

数据库位置:`~/Library/Application Support/com.glean.app/glean.sqlite`

---

## 📄 License

[MIT](./LICENSE) © 2025 Glean Authors

---

## 🙏 致谢 / Acknowledgements

- [Tauri](https://tauri.app/) — 跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) — 渐进式前端框架
- [rusqlite](https://github.com/rusqlite/rusqlite) — Rust SQLite 绑定
- [notify](https://github.com/notify-rs/notify) — 跨平台文件系统通知

---

*项目代号 **Glean**:英文原意"拾穗",指在收割后的田地里捡拾遗留的谷穗 —— 我们从散落的文件中拾取价值。*
