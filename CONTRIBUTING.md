# 贡献指南

感谢你对 Glean 的兴趣！🎉

## 🚀 快速开始

```bash
# 1. Fork & clone
git clone https://github.com/<your-username>/glean.git
cd glean

# 2. 安装依赖
pnpm install

# 3. 启动开发模式
pnpm tauri dev
```

**前置要求**：
- macOS 13+
- Node.js 20+
- pnpm 11+
- Rust 1.75+ (`rustup`)
- Xcode Command Line Tools

## 🛠 开发流程

1. **创建分支**
   ```bash
   git checkout -b feat/your-feature
   # 或
   git checkout -b fix/your-bugfix
   ```

2. **编码 → 提交**
   ```bash
   git commit -m "feat(scope): 简短描述"
   ```
   
   推荐 commit 前缀：
   - `feat`: 新功能
   - `fix`: Bug 修复
   - `docs`: 文档
   - `refactor`: 重构
   - `chore`: 构建 / 工具

3. **本地验证**
   ```bash
   # 前端类型检查
   pnpm exec vue-tsc --noEmit
   
   # Lint
   pnpm lint
   
   # Rust 编译
   cargo check --manifest-path src-tauri/Cargo.toml
   ```

4. **推送 + PR**
   ```bash
   git push origin feat/your-feature
   ```
   然后在 GitHub 创建 PR，描述变更并关联 Issue。

## 📐 代码规范

### Rust
- 跟随 `cargo fmt` 默认风格
- 避免 `unwrap()`，用 `?` 或 `Result` 处理错误
- 公开 API 加 doc comment `///`
- Tauri command 返回 `Result<T, String>` 用于前端友好错误

### TypeScript / Vue
- `<script setup lang="ts">` + Composition API
- 避免任何 `any`，必要时用 `unknown` + 类型守卫
- 文件命名：组件 `PascalCase.vue`，其他 `kebab-case.ts`
- Pinia store 用 setup 风格（`defineStore("xxx", () => {...})`）

### CSS
- Tailwind 优先
- 避免在组件 `<style>` 中写全局样式

## 🧪 测试

当前测试覆盖率较低，欢迎贡献：
- Rust 单元测试：`src-tauri/src/**/*_test.rs`
- 前端组件测试：`src/**/__tests__/*.test.ts`（暂未配置）

## 🌐 国际化

翻译在 `src/locales/{zh-CN,en}.json`，结构对称。新增键时：
1. 在两个 locale 文件中都加
2. 模板中使用 `$t('key')` 或脚本中使用 `const { t } = useI18n()`
3. 命名空间：`section.key`（如 `filelist.empty_all`）

## 📁 项目结构

```
glean/
├── src/                    # Vue 前端
│   ├── components/         #   UI 组件
│   ├── stores/             #   Pinia stores
│   ├── locales/            #   i18n 翻译
│   └── ...
├── src-tauri/src/          # Rust 后端
│   ├── agent/              #   Agent 工具系统
│   ├── db/                 #   SQLite + 迁移
│   ├── llm/                #   LLM Provider
│   ├── rag.rs              #   RAG + Agent loop
│   └── ...
├── docs/                   # VitePress 文档站
└── .github/                # CI + 模板
```

## 🐛 报告 Bug / 建议功能

- [提交 Issue](https://github.com/WalkAlone0325/glean/issues/new/choose)
- 优先选择对应模板（Bug / Feature）

## 💬 交流

- GitHub Discussions（coming soon）
- Issue 用于明确的 bug / 功能请求

## 📄 License

提交的代码将在 [MIT License](./LICENSE) 下发布。

---

再次感谢你的贡献！✨
