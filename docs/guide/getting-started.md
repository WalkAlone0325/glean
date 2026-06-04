# 快速开始

## 下载安装

前往 [Releases](https://github.com/WalkAlone0325/glean/releases) 下载最新版本：

- **Apple Silicon (M1/M2/M3/M4)**: `Glean_<version>_aarch64.dmg`
- **Intel Mac**: `Glean_<version>_x64.dmg`

## 安装步骤

1. 双击打开 `.dmg` 文件
2. 将 Glean 拖入 Applications 文件夹
3. 启动 Glean

### 无法验证开发者？

由于应用当前未签名，首次启动会提示"无法验证开发者"。在终端执行：

```bash
xattr -dr com.apple.quarantine /Applications/Glean.app
```

然后再次启动即可。

## 首次启动

应用启动后会显示欢迎向导：

1. **欢迎页** — 介绍三大核心能力
2. **选择索引文件夹** — 默认扫描 `~/Documents`、`~/Downloads`、`~/Desktop`，可增删
3. **开始索引** — 后台扫描，预计 1-3 分钟（取决于文件数量）

索引完成后即可：

- 用 `⌘K` 打开搜索面板
- 用 `⌘B` 打开 AI 助手
- 用 `⌘⇧Space` 全局唤起/隐藏窗口

## 配置 LLM（可选）

要使用 AI 助手需要配置 LLM Provider：

1. 打开设置（右上角齿轮 / `⌘,`）
2. 选择预设：OpenAI / DeepSeek / 智谱 / Moonshot / 通义
3. 填入 API Key
4. 点击「测试连通性」验证
5. 保存

所有配置存储在本地 SQLite，不上传任何服务器。
