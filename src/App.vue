<script setup lang="ts">
import { Search, Folder, FileText, Settings } from "@lucide/vue";
</script>

<template>
  <div class="flex h-screen w-screen flex-col bg-background text-foreground">
    <header
      class="flex h-12 items-center gap-3 border-b border-border px-4"
      data-tauri-drag-region
    >
      <span class="text-sm font-semibold tracking-tight">Glean</span>
      <div
        class="ml-4 flex flex-1 items-center gap-2 rounded-md bg-muted px-3 py-1.5"
      >
        <Search class="size-4 text-muted-foreground" />
        <input
          class="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          placeholder="搜索文件、内容、命令..."
        />
        <kbd class="text-xs text-muted-foreground">⌘K</kbd>
      </div>
      <button
        class="rounded-md p-1.5 text-muted-foreground hover:bg-muted"
        aria-label="设置"
      >
        <Settings class="size-4" />
      </button>
    </header>

    <div class="flex flex-1 overflow-hidden">
      <aside class="w-56 border-r border-border bg-muted/30 p-3">
        <nav class="space-y-1 text-sm">
          <a class="flex items-center gap-2 rounded-md bg-muted px-2 py-1.5">
            <Folder class="size-4" />
            所有文件
          </a>
          <a
            class="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted"
          >
            <FileText class="size-4" />
            最近查看
          </a>
        </nav>
      </aside>

      <main class="flex-1 overflow-auto p-6">
        <div
          class="flex h-full flex-col items-center justify-center text-muted-foreground"
        >
          <p class="text-sm">尚未索引任何文件夹</p>
          <button
            class="mt-3 rounded-md bg-primary px-4 py-1.5 text-sm text-primary-foreground hover:opacity-90"
          >
            选择文件夹开始索引
          </button>
        </div>
      </main>

      <aside class="w-72 border-l border-border p-4">
        <p class="text-xs text-muted-foreground">详情面板</p>
      </aside>
    </div>
  </div>
</template>
