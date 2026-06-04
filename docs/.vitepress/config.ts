import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Glean",
  description: "本地优先、AI 原生的 macOS 文件管理器",
  lang: "zh-CN",
  lastUpdated: true,
  cleanUrls: true,
  base: "/glean/",
  head: [
    ["link", { rel: "icon", href: "/glean/icon.png" }],
    ["meta", { name: "theme-color", content: "#7c3aed" }],
  ],
  themeConfig: {
    logo: "/icon.png",
    nav: [
      { text: "首页", link: "/" },
      { text: "指南", link: "/guide/getting-started" },
      { text: "功能", link: "/features/search" },
      { text: "FAQ", link: "/faq" },
      {
        text: "GitHub",
        link: "https://github.com/WalkAlone0325/glean",
      },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "入门",
          items: [
            { text: "快速开始", link: "/guide/getting-started" },
            { text: "安装", link: "/guide/installation" },
            { text: "首次启动", link: "/guide/onboarding" },
          ],
        },
        {
          text: "核心功能",
          items: [
            { text: "文件索引", link: "/guide/indexing" },
            { text: "搜索", link: "/guide/search" },
            { text: "AI 助手", link: "/guide/ai-assistant" },
            { text: "标签与收藏", link: "/guide/tags" },
          ],
        },
        {
          text: "参考",
          items: [
            { text: "快捷键", link: "/guide/shortcuts" },
            { text: "设置项", link: "/guide/settings" },
            { text: "隐私", link: "/guide/privacy" },
          ],
        },
      ],
    },
    socialLinks: [
      { icon: "github", link: "https://github.com/WalkAlone0325/glean" },
    ],
    footer: {
      message: "Released under the MIT License.",
      copyright: "Copyright © 2026 Glean Authors",
    },
    outline: { level: [2, 3], label: "本页导航" },
    docFooter: { prev: "上一篇", next: "下一篇" },
    lastUpdatedText: "最后更新",
    returnToTopLabel: "回到顶部",
    sidebarMenuLabel: "菜单",
    darkModeSwitchLabel: "主题",
    lightModeSwitchTitle: "切换到浅色模式",
    darkModeSwitchTitle: "切换到深色模式",
  },
});
