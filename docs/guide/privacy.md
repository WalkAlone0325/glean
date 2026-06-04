# 隐私声明

Glean 是**本地优先**的应用。我们相信你的文件应该属于你。

## 数据存储

所有数据存储在本地 SQLite 数据库：

```
~/Library/Application Support/com.glean.app/glean.sqlite
```

包含：
- 文件元数据（路径、名称、大小、修改时间、类型）
- 文本内容分块（chunks）+ 向量嵌入
- 对话历史
- 标签 / 收藏 / 最近查看记录
- LLM Provider 配置（含 API Key）
- 操作日志（用于撤销）

**没有任何数据上传到我们的服务器** — 因为我们没有服务器。

## LLM 调用

当你使用 AI 助手时：

1. 你的问题和检索到的文件片段会发送到**你配置的** LLM Provider（OpenAI / DeepSeek / 智谱等）
2. API Key 仅用于调用 Provider API，不传输给第三方
3. Provider 的数据处理受其隐私政策约束

**不使用 AI 助手时，没有任何外部网络请求。**

## 遥测

**零遥测**。我们不收集：
- 使用统计
- 崩溃报告
- 设备指纹
- 用户行为

如果未来需要遥测，会**默认关闭、明确告知、可一键关闭**。

## 文件系统访问

Glean 仅扫描你**明确授权**的文件夹。默认忽略：

- `.git` / `node_modules` / `.Trash` / `.DS_Store`
- 大于 100MB 的文件
- 你在设置中添加的自定义规则（`.gleanignore` 语法）

## 删除数据

卸载应用后，删除以下目录即可清除所有数据：

```bash
rm -rf ~/Library/Application\ Support/com.glean.app/
```

或在应用内：设置 → 索引管理 → 取消索引 → 移除文件夹。
