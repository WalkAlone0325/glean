<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Search, Folder, FileText, Settings, Loader2, FolderOpen } from "@lucide/vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "./stores/app";
import { useSearchStore } from "./stores/search";
import SearchPalette from "./components/SearchPalette.vue";

const app = useAppStore();
const search = useSearchStore();
const indexing = ref(false);

async function pickAndIndex() {
  const selected = await openDialog({ directory: true, multiple: false });
  if (typeof selected !== "string") return;
  indexing.value = true;
  try {
    await invoke("start_indexing", { paths: [selected] });
    app.indexedFolders = Array.from(new Set([...(app.indexedFolders || []), selected]));
  } catch (e) {
    console.error(e);
  } finally {
    indexing.value = false;
  }
}

onMounted(async () => {
  await app.bootstrap();
  await listen<{ total: number; duration_ms: number }>("index-complete", async () => {
    indexing.value = false;
    await app.refreshStats();
  });
  await listen<{ current: number; total: number; phase: string }>("index-progress", (e) => {
    if (e.payload.phase === "Indexing") indexing.value = true;
    if (e.payload.phase === "Completed" || e.payload.phase === "Cancelled") indexing.value = false;
  });
});
</script>

<template>
  <div class="flex h-screen w-screen flex-col bg-background text-foreground">
    <header class="flex h-12 items-center gap-3 border-b border-border px-4" data-tauri-drag-region>
      <span class="text-sm font-semibold tracking-tight">Glean</span>
      <button
        @click="search.paletteOpen = true"
        class="ml-4 flex flex-1 items-center gap-2 rounded-md bg-muted px-3 py-1.5 text-left text-sm text-muted-foreground transition hover:bg-muted/80"
      >
        <Search class="size-4" />
        <span class="flex-1">搜索文件、内容、命令...</span>
        <kbd class="text-xs">⌘K</kbd>
      </button>
      <button class="rounded-md p-1.5 text-muted-foreground hover:bg-muted" aria-label="设置">
        <Settings class="size-4" />
      </button>
    </header>

    <div class="flex flex-1 overflow-hidden">
      <aside class="w-56 border-r border-border bg-muted/30 p-3">
        <nav class="space-y-1 text-sm">
          <a class="flex items-center gap-2 rounded-md bg-muted px-2 py-1.5">
            <Folder class="size-4" />
            所有文件
            <span class="ml-auto text-xs text-muted-foreground">{{ app.stats.files }}</span>
          </a>
          <a class="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted">
            <FileText class="size-4" />
            最近查看
          </a>
        </nav>
        <div class="mt-6">
          <div class="mb-2 px-2 text-xs uppercase tracking-wide text-muted-foreground">
            已索引文件夹
          </div>
          <div v-if="!app.indexedFolders?.length" class="px-2 text-xs text-muted-foreground">
            尚未索引任何文件夹
          </div>
          <ul v-else class="space-y-1 text-xs">
            <li
              v-for="f in app.indexedFolders"
              :key="f"
              class="truncate rounded-md px-2 py-1 text-muted-foreground"
              :title="f"
            >
              {{ f }}
            </li>
          </ul>
        </div>
      </aside>

      <main class="flex-1 overflow-auto p-6">
        <div
          v-if="!app.indexedFolders?.length"
          class="flex h-full flex-col items-center justify-center text-muted-foreground"
        >
          <p class="text-sm">还没有索引任何文件夹</p>
          <button
            @click="pickAndIndex"
            :disabled="indexing"
            class="mt-3 flex items-center gap-2 rounded-md bg-primary px-4 py-1.5 text-sm text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
          >
            <FolderOpen class="size-4" />
            选择文件夹开始索引
          </button>
        </div>
        <div v-else class="flex h-full flex-col">
          <div class="mb-4 flex items-center justify-between">
            <div>
              <h2 class="text-base font-semibold">已索引 {{ app.stats.files }} 个文件</h2>
              <p class="text-xs text-muted-foreground">FTS + jieba-rs 中文分词已启用</p>
            </div>
            <button
              @click="pickAndIndex"
              :disabled="indexing"
              class="flex items-center gap-2 rounded-md bg-muted px-3 py-1.5 text-xs transition hover:bg-muted/80 disabled:opacity-50"
            >
              <Loader2 v-if="indexing" class="size-3 animate-spin" />
              <FolderOpen v-else class="size-3.5" />
              {{ indexing ? "索引中..." : "添加文件夹" }}
            </button>
          </div>
          <div class="flex flex-1 items-center justify-center rounded-lg border border-dashed border-border text-sm text-muted-foreground">
            按 <kbd class="mx-1 rounded bg-muted px-1 text-xs">⌘K</kbd> 开始搜索
          </div>
        </div>
      </main>

      <aside class="w-72 border-l border-border p-4">
        <p class="text-xs text-muted-foreground">详情面板</p>
      </aside>
    </div>

    <SearchPalette />
  </div>
</template>
