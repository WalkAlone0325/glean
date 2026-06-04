---
layout: home

hero:
  name: Glean
  text: 本地优先的 AI 文件管家
  tagline: 从散落的文件中拾取价值 — 找得到、记得住、自动归档
  image:
    src: /hero.png
    alt: Glean 主界面
  actions:
    - theme: brand
      text: 下载 macOS
      link: https://github.com/WalkAlone0325/glean/releases
    - theme: alt
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/WalkAlone0325/glean

features:
  - icon: 🔍
    title: 语义检索
    details: 用自然语言找文件 — 「上周那个讲融资的 PDF 在哪？」RAG + BM25 + 向量 KNN 混合排序。
  - icon: 🤖
    title: AI 助手
    details: 对话式检索文件、自动打标签、整理重命名。Agent 工具调用支持撤销与确认。
  - icon: 🏷
    title: 标签与收藏
    details: 多彩标签、星标文件、最近查看、智能文件夹 — 让文件有归属。
  - icon: 🔒
    title: 本地优先
    details: 所有索引、对话、配置存于本地 SQLite。BYOK 调用 LLM，零服务器依赖。
  - icon: ⚡
    title: 性能优先
    details: Rust 后端 + Tauri v2 + 虚拟列表，启动 &lt;1s，内存 &lt;200MB。
  - icon: 🌐
    title: 中英双语
    details: 完整国际化支持，跟随系统语言或手动切换。
---

<!-- trigger first deploy -->
