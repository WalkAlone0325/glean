# 常见问题

## 安装与启动

### 提示"应用已损坏"或"无法验证开发者"

由于应用未签名，macOS 会阻止启动。在终端执行：

```bash
xattr -dr com.apple.quarantine /Applications/Glean.app
```

### 启动后没有文件

Glean 需要先索引文件夹。检查：

1. 是否完成了首次启动向导？
2. 设置 → 索引管理 中是否显示已索引目录？
3. 点击「开始索引」手动启动

### macOS Ventura (13.0) 以下能否使用？

不支持。Glean 使用了一些 macOS 13+ 的 API（Vision OCR 等）。

## 搜索

### 搜索结果不准？

可能原因：

1. **索引未完成**：查看右上角进度条
2. **向量未生成**：首次索引后需等待 embedding 完成
3. **文件被忽略**：检查 `.gleanignore` 规则
4. **查询太短**：单字查询效果较差，建议 2-3 个词

### PDF 内容搜不到？

PDF 需要先做 OCR。Glean 使用 macOS Vision Framework 处理扫描型 PDF。

### 可以搜图片内容吗？

当前版本仅支持图片元数据（文件名、路径）。多模态内容理解在 Phase 5 规划中。

## AI 助手

### 报错"请先填入 API Key"

打开设置 → LLM Provider → 填入 API Key → 测试连通性。

### 响应很慢

可能原因：
1. LLM Provider 网络延迟（建议国内用户使用 DeepSeek / 智谱）
2. RAG 检索了大量上下文（可在聊天面板关闭 RAG）
3. 模型本身较慢（尝试 gpt-4o-mini 等轻量模型）

### 工具调用失败

查看工具卡片（可展开），常见错误：

- `file not indexed`: 文件不在索引中，需先扫描
- `file too large`: 超过 2MB 预览上限
- `destination already exists`: 移动文件时目标已存在，需 overwrite=true

## 性能

### 内存占用高？

向量索引在内存中缓存以加速检索。如果文件很多（>10 万），内存占用会上升。

### 索引很慢？

首次索引最慢。后续增量索引只扫描变化的文件，很快。

可以暂停索引：设置 → 索引管理 → 暂停。

## 数据

### 如何备份数据？

复制整个数据目录：

```bash
cp -r ~/Library/Application\ Support/com.glean.app/ /backup/
```

### 如何重置所有数据？

```bash
rm -rf ~/Library/Application\ Support/com.glean.app/
```

下次启动会被当作首次运行。

---

没找到答案？[提交 Issue](https://github.com/WalkAlone0325/glean/issues/new)。
