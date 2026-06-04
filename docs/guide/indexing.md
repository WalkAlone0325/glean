# 文件索引

Glean 通过扫描你授权的目录建立本地索引。所有数据存储在 SQLite。

## 工作流程

```
扫描目录 → 遍历文件 → 提取元数据 → 文本分块 → 嵌入向量（可选）
```

每个步骤都可暂停/恢复。

## 支持的文件类型

| 类型 | 提取方式 |
|------|---------|
| 文本（txt/md/json/csv 等） | 直接读取 |
| 代码（rs/ts/py/go 等） | 直接读取 + 语法识别 |
| PDF | 文本层 + Vision OCR（扫描型 PDF） |
| 图片（jpg/png/heic 等） | EXIF + Vision（暂未启用内容识别） |
| Office（docx/xlsx/pptx） | 提取内嵌文本 |
| HTML / XML | 提取文本 |

## 自动忽略

内置忽略规则（不可关闭）：

```
.git/  node_modules/  .Trash/  .DS_Store
.npm/  .pnpm-store/  .cache/  target/  dist/  build/
.next/  .nuxt/  .svelte-kit/  .turbo/  .vercel/  .parcel-cache/
Coverage/  __pycache__/  .pytest_cache/  .mypy_cache/
.venv/  venv/  .idea/  .vscode/  .history/
Library/  .Trash-0/
```

文件大小上限 **100MB**。

## 自定义忽略规则

设置 → 忽略规则，每行一个模式：

```
*.log
*.tmp
tmp/
*.cache
~/Projects/experimental/
```

支持 glob 通配符（`*` `?`）。

## 文件监听

索引完成后，Glean 通过 `notify`（FSEvents）监听文件变化：

- 新建文件 → 自动加入索引
- 修改文件 → 重新提取内容
- 删除文件 → 标记 `deleted_at`（不立即删除，便于撤销）

## 性能

| 指标 | 数值 |
|------|------|
| 首次扫描速度 | 约 5000 文件/分钟（SSD） |
| 增量扫描 | 仅扫描变化，秒级完成 |
| 内存占用 | < 200MB |
| 索引大小 | 约原文本大小的 1.5 倍 |

## 暂停/恢复

索引会消耗 CPU 和磁盘 IO。如需暂停：

**设置 → 索引管理 → 暂停** 或工具栏暂停按钮

也可在终端关闭后台 IO 限制：

```bash
# 提升后台任务优先级（仅高级用户）
sudo taskpolicy -c utility
```
