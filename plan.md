# Glean 开发计划

> **本地优先、AI 原生的 macOS 文件管理器**  
> 项目代号：**Glean**（含义：拾穗、收集、慢慢积累）  
> 从你电脑里散落的文件中拾取价值。

---

## 📋 整体路线图

| 阶段 | 时间 | 目标 | 交付物 |
|------|------|------|--------|
| **Phase 0** | 1 周 | 项目地基 | 可运行的 Tauri 空壳 |
| **Phase 1** | 3 周 | 文件索引核心 | 能扫描 + 全文检索 |
| **Phase 2** | 3 周 | 语义检索 | 自然语言查文件 |
| **Phase 3** | 3 周 | Agent 能力 | 多轮对话 + 任务执行 |
| **Phase 4** | 2 周 | 打磨 + 分发 | 公测版 v0.1 |
| **Phase 5** | 持续 | 增长 + 商业化 | v1.0 |

**总周期**：3-4 个月 MVP 上线，6 个月达到可商用水平。

---

## 🏗 Phase 0：项目地基（第 1 周）

**目标**：搭建开发环境，跑通最小骨架。

### Day 1-2：环境与项目初始化

- [ ] 安装 Rust（`rustup`）、Node.js 20+、Xcode CLI Tools
- [ ] `npm create tauri-app@latest` 初始化项目（**选 Vue + TypeScript**）
- [ ] 配置 `rust-analyzer`、`eslint`、`prettier`、`vue-tsc`
- [ ] 创建 GitHub 仓库，配置 `.gitignore`（排除 `.DS_Store`、SQLite 文件）
- [ ] 配置 GitHub Actions：PR 检查 + Release 自动打包

### Day 3-4：核心依赖引入

**Rust 端**：
- [ ] `rusqlite`（带 `bundled` feature，免去系统依赖）
- [ ] `rusqlite_migration`（轻量迁移）
- [ ] `notify`（FSEvents 监听）
- [ ] `walkdir`（文件遍历）
- [ ] `tokio`（异步运行时）
- [ ] `serde` + `serde_json`（序列化）
- [ ] `anyhow` + `thiserror`（错误处理）
- [ ] `tracing` + `tracing-subscriber`（日志）
- [ ] `reqwest`（HTTP，LLM 调用）
- [ ] `xxhash-rust`（文件哈希，比 SHA 快 10 倍）

**前端**：
- [ ] **Vue 3.5+**（`<script setup>` + Props 解构）
- [ ] **Vite 6+** + **Vue Router 4**
- [ ] **Pinia 3**（状态管理）+ `pinia-plugin-persistedstate`
- [ ] **VueUse**（`@vueuse/core`，必备）
- [ ] **Tailwind CSS v4**（oxide 引擎）
- [ ] **shadcn-vue**（UI 组件，已支持 Tailwind v4）
- [ ] **lucide-vue-next**（图标）
- [ ] **@tanstack/vue-query**（数据请求）
- [ ] **@tanstack/vue-virtual**（虚拟列表）

**Tauri 插件**：
- [ ] `tauri-plugin-store`（配置存储）
- [ ] `tauri-plugin-dialog`（文件选择）
- [ ] `tauri-plugin-fs`（文件操作）
- [ ] `tauri-plugin-global-shortcut`（全局快捷键）
- [ ] `tauri-plugin-window-state`（窗口位置记忆）
- [ ] `tauri-plugin-clipboard-manager`（剪贴板）
- [ ] `tauri-plugin-updater`（自动更新）
- [ ] `tauri-plugin-log`（日志）

### Day 5：基础 UI 框架

- [ ] 三栏布局骨架：左侧文件夹树 / 中间文件列表 / 右侧详情
- [ ] 全局快捷键唤起（`Cmd+Shift+Space`，避开 Spotlight）
- [ ] 系统托盘（菜单栏图标）
- [ ] 基础主题（深色/浅色，跟随系统）
- [ ] shadcn-vue 初始化：`npx shadcn-vue@latest init`

### Day 6-7：核心数据模型

- [ ] 设计 SQLite Schema：
  ```sql
  files (id, path, name, ext, size, mtime, hash, kind, indexed_at)
  chunks (id, file_id, content, embedding_id, page, position)
  tags (id, name, color)
  file_tags (file_id, tag_id)
  notes (id, file_id, content, created_at)
  conversations (id, title, created_at, updated_at)
  messages (id, conversation_id, role, content, tokens, created_at)
  ```
- [ ] 数据库迁移（`rusqlite_migration`，版本化管理）
- [ ] Tauri Command 桥接：`get_files`、`search_files` 等
- [ ] 全局 SQLite 连接池（`Mutex<Connection>` 或 `r2d2`）

**✅ 验证标准**：能启动 App，能在 UI 看到一个空列表，能从 Rust 读到 SQLite 数据。

---

## 📂 Phase 1：文件索引核心（第 2-4 周）

**目标**：用户选择文件夹后，自动扫描 + 实时监听 + 全文检索。

### Week 2：文件扫描器

- [ ] 文件遍历引擎（`walkdir` + tokio 异步任务）
  - 支持 `~/Desktop`、`~/Documents`、`~/Downloads`、自定义路径
  - **智能忽略规则**：`.git/`、`node_modules/`、`.Trash`、`.DS_Store`、iCloud 占位符 `.icloud`、大于 100MB 的文件
  - 支持用户自定义忽略规则（`.gleanignore` 文件，语法兼容 `.gitignore`）
- [ ] 文件元数据提取
  - 基础：大小、扩展名、创建/修改时间
  - macOS 增强：调用 `mdls` / `NSMetadataItem` 拿到 Spotlight 元数据（EXIF、作者、PDF 标题）
- [ ] **文件哈希**：`xxhash-rust`（xxh3 算法，比 SHA 快 10 倍）
- [ ] **索引调度器**
  - 后台任务队列（`tokio::sync::mpsc`）
  - 优先级：用户当前查看的文件夹 > 最近修改 > 历史回填
  - 限速（`taskpolicy -c background` 启动后台 IO）
- [ ] 进度反馈：实时向 UI 推送索引进度（Tauri Event）

### Week 3：文件监听 + 增量更新

- [ ] 使用 `notify` crate 监听 FSEvents
  - 新增文件 → 入队索引
  - 修改文件 → 重新索引该文件
  - 删除文件 → 标记软删除（保留 30 天后物理删除）
- [ ] 处理大批量变更（如 `git checkout` 触发上千文件变化，需要 debounce）
- [ ] 重启后的"一致性检查"（对比数据库 vs 文件系统）
- [ ] **资源监控**：索引时显示 CPU/磁盘占用，用户可暂停
- [ ] **日志系统**：`tracing` + 文件轮转，便于排查问题

### Week 4：全文检索（FTS）

- [ ] SQLite FTS5 集成
  - **中文分词**：`jieba-rs`（必须，默认 tokenizer 中文不行）
  - 同步更新：文件索引后写入 FTS 表
- [ ] 搜索 API：支持短语、排除、文件类型过滤
- [ ] 前端搜索框 + 结果高亮 + 摘要（snippet）
- [ ] **命令面板**（Cmd+K）：`vue-cmdk` 或 shadcn-vue 命令面板模板
- [ ] **杀手功能 1**：结果点击直接打开文件（`open` 命令）
- [ ] **杀手功能 2**：PDF 跳转到具体页、Markdown 跳转到具体行

**✅ 验证标准**：扫描 `~/Downloads` 5000 个文件，能在 100ms 内搜出"包含 XX 的 PDF"。

---

## 🧠 Phase 2：语义检索（第 5-7 周）

**目标**：能回答"上周那个讲融资的 PDF 在哪"，能跨文件关联。

### Week 5：Embedding 集成

- [ ] 评估并选择本地 embedding 模型
  - **首选：`Qwen3-Embedding-0.6B`**（2025 末发布，中文 MTEB 霸榜）
  - **备选：`mlx-community/bge-m3-mlx`**（多语言强，生态成熟）
  - 不推荐：nomic-embed（中文弱）
- [ ] **MLX Rust binding**（`mlx-rs`，2025 已成熟）
  - 备选：调用 Python sidecar（不推荐，依赖 Python 环境）
  - 备选：`candle`（纯 Rust，但模型兼容性差）
- [ ] 模型下载管理
  - 首次启动提示下载（约 1-2GB）
  - 支持手动放置模型文件（不强制下载）
  - 进度条 + 断点续传（`reqwest` 流式）
- [ ] 向量化管线
  - 文档切片（chunk）：PDF 按页 + 段落、Markdown 按 heading、代码按函数
  - batch embedding（提升吞吐）
  - 速率控制（避免 M 芯片过热降频）

### Week 6：向量存储与查询

- [ ] 集成 **LanceDB**（多模态、版本化、Rust 原生）
  - 表结构：`chunks_v1(file_id, chunk_text, vector, page, position)`
  - 备选：sqlite-vec（更简单，但功能少）
- [ ] 混合检索：向量相似度 + BM25（FTS5）+ 元数据过滤
  - 使用 **RRF（Reciprocal Rank Fusion）** 融合两路结果
- [ ] 重排序（可选）：`bge-reranker-v2-m3` 提升精度
- [ ] 查询性能优化：HNSW 索引、量化（PQ/SQ）

### Week 7：自然语言检索 UI

- [ ] 单一搜索框：自动判断"关键词查询"还是"自然语言查询"
- [ ] 查询改写（可选）：用 LLM 把口语化查询改成关键词
  - "上周那个融资 PDF" → `融资 OR valuation date:>2026-05-27 type:pdf`
- [ ] 结果展示优化
  - 文件预览缩略图（macOS QuickLook API，免费）
  - 高亮匹配段落 + 周围上下文
  - 多结果并排对比
- [ ] **OCR 支持**（关键差异化）
  - 调用 macOS Vision Framework（`VNRecognizeTextRequest`）
  - 索引截图（`~/Pictures/Screenshots`）、扫描版 PDF
  - 多语言识别（中英混合，精度比 Tesseract 高 5 倍）

**✅ 验证标准**：能搜"那张白板上写着 OKR 的截图"，能搜"上个月合同里甲方公司叫什么"。

---

## 🤖 Phase 3：Agent 能力（第 8-10 周）

**目标**：从"搜索引擎"升级为"会做事的助手"。

### Week 8：LLM 抽象层 + BYOK

- [ ] 统一的 LLM Provider 接口
  ```rust
  #[async_trait]
  trait LLMProvider: Send + Sync {
      async fn chat(&self, messages: Vec<Message>) -> Result<String>;
      async fn stream(&self, messages: Vec<Message>) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>>;
      async fn function_call(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ToolCall>;
  }
  ```
- [ ] 实现 5 个 Provider：
  - **Ollama**（本地，免费）
  - **MLX**（本地，免费，苹果原生，通过 `mlx-rs` 或 lm-sys）
  - **OpenAI**（用户提供 Key，用 `async-openai` crate）
  - **Anthropic Claude**（用户提供 Key，手写或用 `anthropic-sdk`）
  - **DeepSeek / 通义 / 智谱**（兼容 OpenAI 协议，复用 `async-openai`）
- [ ] **LLM 编排推荐**：`rig-core`（可选，统一抽象层，2025 已成熟）
- [ ] 设置页：用户填 Key、选择默认模型、测试连通性
- [ ] 用量统计（本地记录，不上报）
- [ ] **隐私模式开关**：禁用所有云端调用

### Week 9：对话界面 + RAG

- [ ] 聊天 UI（流式输出、Markdown 渲染、代码高亮）
- [ ] **Markdown 渲染**：`markdown-it` + **`shiki`**（VS Code 同款高亮）
  - 或开箱即用：`md-editor-v3`
- [ ] **RAG 管线**
  - 用户提问 → 检索相关 chunks → 拼接 context → LLM 回答
  - 引用展示：每个回答标注来源文件 + 页码，可点击跳转
- [ ] **PDF 预览**：`vue-pdf-embed`（pdf.js 包装）或调用 macOS PDFKit
- [ ] 多轮对话上下文管理（自动摘要压缩）
- [ ] 对话历史持久化（SQLite `conversations` + `messages` 表）
- [ ] **流式动画**：`motion-v` 或 `@formkit/auto-animate`

### Week 10：Agent 工具调用

- [ ] 工具系统设计（OpenAI function calling 协议 + Claude tool use 协议）
  - 内置工具：
    - `search_files(query, filters)` —— 调用本地搜索引擎
    - `read_file(path, page?)` —— 读取文件内容
    - `open_file(path)` —— 在 Finder/默认应用打开
    - `move_file(src, dst)` —— 移动文件（需用户确认）
    - `tag_file(path, tags)` —— 打标签
    - `list_similar(path)` —— 找相似文件
- [ ] **任务循环**（plan → execute → verify）
  - 用户："把 Downloads 里所有发票 PDF 移到 `~/Documents/发票/`"
  - Agent：先 search → 展示给用户确认 → 批量执行
- [ ] **安全沙箱**：危险操作必须用户确认（删除、移动、批量改名）
- [ ] **撤销栈**：所有 Agent 执行的操作支持撤销（操作日志表）

**✅ 验证标准**：能完成"找文件 + 整理 + 重命名"的完整闭环，且操作可撤销。

---

## ✨ Phase 4：打磨 + 分发（第 11-12 周）

**目标**：让产品"看起来像产品"，准备公开发布。

### Week 11：体验打磨

- [ ] **冷启动优化**（关键！）
  - 首次启动向导：选择索引文件夹（默认 Desktop + Downloads + Documents）
  - 索引时就有进度条 + "已经能搜到 X 个文件"
  - 第一屏价值：扫描完成后立刻给"3 个重复文件、5 个大文件可清理"
- [ ] **核心功能补全**
  - 文件预览面板（PDF、Markdown、代码、图片、文本）
  - 标签系统（创建、上色、批量打标签）
  - 收藏夹 / 智能文件夹（保存搜索条件）
  - 最近查看 / 经常查看
- [ ] **快捷键体系**
  - 全局唤起、搜索、跳转、确认、取消、命令面板（`Cmd+K`）
- [ ] **设置页**：模型、文件夹、忽略规则、快捷键、隐私
- [ ] **国际化**：`vue-i18n` v9+，中英双语
- [ ] 性能优化
  - 虚拟列表（`@tanstack/vue-virtual`，大文件列表不卡）
  - 启动速度 < 1s
  - 内存占用 < 200MB
- [ ] **稳定性**
  - 全局 panic 处理
  - 数据库定期 VACUUM
  - 崩溃后自动恢复

### Week 12：分发准备

- [ ] **代码签名 + 公证**（macOS 必须）
  - 需要 Apple 开发者账号（$99/年）—— 唯一可能的开销
  - 替代方案：不签名，用户自行 `xattr -d` 解除隔离（早期可接受）
- [ ] **打包**：`.dmg` + `.zip`（Tauri 自带）
- [ ] **自动更新**：Tauri Updater + GitHub Releases 签名
- [ ] **首页/落地页**
  - GitHub Pages（`username.github.io/glean`）—— 不买域名
  - 用 **VitePress** 搭建文档站（Vue 生态原生）
  - 内容：截图 + GIF + 核心卖点 + 下载链接 + FAQ
- [ ] **README / 文档**
  - 中英双语 README
  - 录 30 秒演示视频（QuickTime 录屏，免费）
  - 截图集（用 `shottr` 或 macOS 自带）
- [ ] **分发渠道提交**
  - GitHub Release（创建 v0.1.0 tag）
  - Homebrew Cask PR（需要先有一定 star）
  - 少数派 / V2EX / 即刻 / 小红书 / B 站发帖
  - Product Hunt 提交（英文市场）
  - Hacker News（Show HN）

**✅ 验证标准**：陌生用户能下载、安装、5 分钟内感受到价值。

---

## 🚀 Phase 5：增长与商业化（持续）

### 短期（v0.2 - v0.5，第 4-5 个月）

- [ ] 用户反馈通道（GitHub Issues 模板）
- [ ] 错误报告（用户主动触发，附日志，**不自动上报**）
- [ ] 多模态：图片内容理解（CLIP / SigLIP 本地模型）
- [ ] 浏览器集成：索引 Chrome/Safari 历史 + 书签
- [ ] 邮件集成：导入 Apple Mail / Outlook 邮件
- [ ] **关系图谱**：文件之间的引用关系可视化（用 `vis-network` 或 D3）
- [ ] 插件系统（让社区扩展）

### 中期（v1.0，第 6 个月）

- [ ] iOS/iPadOS 配套 App（用 CloudKit 同步，**零服务器**）
- [ ] 团队版（共享索引，可选）
- [ ] 企业版（本地部署、SSO、审计日志）

### 商业化路径（达到 1000+ 活跃用户后）

- [ ] **爱发电** 接受赞助（最低门槛）
- [ ] **Lemon Squeezy** 卖 License Key（一次性买断 ¥199-499）
- [ ] 分免费版 / Pro 版（Pro：高级模型、多设备同步、插件）
- [ ] **永远保留免费版**，这是隐私叙事的核心

---

## 📊 关键技术决策（2026 版）

### 前端栈（Vue 3 生态）

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 框架 | **Vue 3.5+** | `<script setup>` + Props 解构 |
| 构建 | **Vite 6+** | Rolldown 集成 |
| 状态 | **Pinia 3** | Vue 官方标配 |
| 路由 | **Vue Router 4** | 必要时启用 |
| UI 组件 | **shadcn-vue** | 现代美学，源码可控 |
| 样式 | **Tailwind CSS v4** | oxide 引擎 |
| 图标 | **lucide-vue-next** | shadcn 默认搭配 |
| Hook 库 | **VueUse** | 必备 |
| 数据请求 | **@tanstack/vue-query** | 缓存/重试 |
| 虚拟列表 | **@tanstack/vue-virtual** | 大列表 |
| Markdown | **markdown-it + shiki** | VS Code 同款高亮 |
| PDF 预览 | **vue-pdf-embed** | pdf.js 包装 |
| 动画 | **motion-v / auto-animate** | Framer Motion 替代 |
| 国际化 | **vue-i18n v9+** | 中英双语 |
| 文档站 | **VitePress** | Vue 生态原生 |

### 后端栈（Rust + Tauri）

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 桌面壳 | **Tauri v2** | 体积小，Rust 原生 |
| 数据库 | **SQLite + FTS5** | 单文件，零运维 |
| SQLite 驱动 | **rusqlite (bundled)** | 同步 API 简单 |
| 迁移 | **rusqlite_migration** | 轻量 |
| 向量库 | **LanceDB** | 多模态、版本化 |
| 中文分词 | **jieba-rs** | Rust 唯一选择 |
| 文件哈希 | **xxhash-rust (xxh3)** | 比 SHA 快 10 倍 |
| 文件监听 | **notify (FSEvents)** | macOS 原生 |
| 异步运行时 | **tokio** | 标准 |
| HTTP 客户端 | **reqwest** | 标准 |
| LLM 客户端 | **async-openai + 手写 Claude** | OpenAI 协议覆盖 90% |
| LLM 编排 | **rig-core**（可选） | 多 provider 抽象 |
| 日志 | **tracing + tracing-subscriber** | 结构化日志 |
| 错误处理 | **anyhow + thiserror** | 标配 |

### AI 能力

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 本地 LLM | **MLX**（mlx-rs） | 苹果原生，最快 |
| 本地 Embedding | **Qwen3-Embedding-0.6B** | 中文最强 |
| 备选 Embedding | **bge-m3** | 多语言，生态成熟 |
| 重排序 | **bge-reranker-v2-m3** | 可选 |
| OCR | **Vision Framework** | 免费、准、快 |
| 云端 LLM | **多 Provider + BYOK** | 不绑定厂商 |

### 分发与商业化

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 分发 | **GitHub Releases + Homebrew** | 完全免费 |
| 自动更新 | **Tauri Updater** | 官方方案 |
| 官网 | **GitHub Pages + VitePress** | ¥0 |
| 支付 | **Lemon Squeezy / 爱发电** | 不需要公司主体 |

---

## 💰 成本结构（目标：¥0/月）

| 环节 | 方案 | 成本 |
|------|------|------|
| 服务器 | 无，纯本地 | ¥0 |
| 域名 | 用 `username.github.io/glean` | ¥0 |
| 数据库 | SQLite 本地文件 | ¥0 |
| LLM | BYOK 或本地模型 | ¥0 |
| 模型分发 | HuggingFace 托管 | ¥0 |
| 应用分发 | GitHub Releases | ¥0 |
| 自动更新 | Tauri Updater + GitHub | ¥0 |
| 文档站 | GitHub Pages + VitePress | ¥0 |
| 支付（未来） | Lemon Squeezy / 爱发电 | 仅按交易抽成 |

**唯一可能的开销**：Apple 开发者账号 $99/年（用于代码签名，可延后到有收入后）。

---

## ⚠️ 容易踩的坑（提前预警）

1. **不要做云端账号系统** —— 一做就要服务器、要合规、要隐私协议，无穷麻烦
2. **不要收集用户数据** —— 任何"匿名遥测"都会破坏隐私叙事
3. **不要绑定单一 LLM 厂商** —— 政策风险，且违背用户期望
4. **不要过早做插件系统** —— 没用户时插件生态起不来，浪费时间
5. **不要做团队协作功能** —— 一做就需要服务端，偏离"个人工具"定位
6. **代码签名可以延后** —— 早期用户能接受 `xattr -d`，等有收入再付 $99
7. **不要用 Electron** —— 内存占用大、Mac 用户普遍反感
8. **不要做完美主义 MVP** —— 第一版丑点没关系，能解决问题就行
9. **Vue 3 必须用 3.5+** —— Props 解构响应式、`useTemplateRef` 极大提升 DX
10. **shadcn-vue 已支持 Tailwind v4** —— 不要按 v3 的旧教程配置

---

## 🎯 一个人开发的优先级心法

**每周问自己三个问题**：

1. 这周做的事，用户能感知到吗？（感知不到 → 砍）
2. 这周做的事，能写进发版说明吗？（写不进去 → 砍）
3. 这周做的事，是用户痛点还是我的技术痒？（技术痒 → 砍）

砍掉一切"开发者觉得牛逼但用户用不上"的功能。

---

## 📅 第一周立刻开始的具体动作

1. **今天**：`npm create tauri-app@latest`（选 Vue + TS），跑通 hello world
2. **明天**：把 SQLite 集成进去，能写一条记录能读出来
3. **后天**：写一个简单的文件扫描器，扫描 `~/Downloads` 并写入数据库
4. **第四天**：在前端把扫描结果显示出来（用 VueUse + Pinia）
5. **第五天**：加一个简单的搜索框（先不上 FTS）
6. **第六-七天**：初始化 shadcn-vue，写一个发版 README

**先跑通"扫描 - 存储 - 查询"的最小闭环，再考虑 embedding 和 LLM**。

---

## 🔗 关键链接（待用）

- Tauri v2 文档：https://v2.tauri.app/
- Vue 3 文档：https://vuejs.org/
- shadcn-vue：https://www.shadcn-vue.com/
- VueUse：https://vueuse.org/
- LanceDB：https://lancedb.github.io/lancedb/
- mlx-rs：https://github.com/oxideai/mlx-rs（或官方苹果 mlx-swift 的 Rust binding）
- async-openai：https://github.com/64bit/async-openai
- rig：https://github.com/0xPlaygrounds/rig

---

*文档创建于 2026-06-03*  
*项目代号：Glean*  
*技术栈：Tauri v2 + Vue 3 + Rust + MLX + LanceDB*  
*仓库地址：待定*
