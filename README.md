# Glean

> 本地优先、AI 原生的 macOS 文件管理器  
> 从你电脑里散落的文件中拾取价值。

---

## ✨ 项目定位

Glean 是一个 **macOS 原生桌面应用**，结合本地 Embedding、向量检索与 LLM Agent，解决"我电脑里有 5000 个文件，我知道我看过，但我找不到"的痛点。

**核心能力**：

- 🗂 **智能索引**：扫描 ~/Documents、~/Downloads、~/Desktop，理解 PDF / Markdown / 图片 / 邮件
- 🔍 **语义检索**：自然语言查询，"上周那个讲融资的 PDF 在哪"
- 🤖 **Agent 执行**：找文件 + 整理 + 重命名 + 自动归档
- 🔒 **本地优先**：所有数据本地存储，BYOK 调用 LLM，零服务器依赖

## 🛠 技术栈

| 层 | 技术 |
|----|------|
| 桌面壳 | Tauri v2 |
| 前端 | Vue 3.5 + TypeScript + Vite 6 |
| UI | shadcn-vue + Tailwind CSS v4 |
| 状态 | Pinia 3 + VueUse |
| 后端 | Rust |
| 数据库 | SQLite + FTS5 |
| 向量库 | LanceDB |
| 本地 LLM | MLX |
| 本地 Embedding | Qwen3-Embedding / bge-m3 |
| OCR | macOS Vision Framework |

## 📦 安装与运行

**前置依赖**：

- macOS 13+
- Node.js 20+
- pnpm 11+
- Rust 1.75+

**开发模式**：

```bash
pnpm install
pnpm tauri dev
```

**生产构建**：

```bash
pnpm tauri build
```

## 📚 文档

- [开发计划](./plan.md) —— 完整路线图与每周里程碑
- [Tauri v2 文档](https://v2.tauri.app/)
- [Vue 3 文档](https://vuejs.org/)

## 🗺 路线图

| 阶段 | 时间 | 状态 |
|------|------|------|
| Phase 0：项目地基 | 第 1 周 | 🚧 进行中 |
| Phase 1：文件索引核心 | 第 2-4 周 | ⏳ 待开始 |
| Phase 2：语义检索 | 第 5-7 周 | ⏳ 待开始 |
| Phase 3：Agent 能力 | 第 8-10 周 | ⏳ 待开始 |
| Phase 4：打磨 + 分发 | 第 11-12 周 | ⏳ 待开始 |

详见 [plan.md](./plan.md)。

## 📄 License

待定（MIT / Apache-2.0 候选）

---

*项目代号：Glean（含义：拾穗、收集、慢慢积累）*
