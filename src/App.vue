<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Search, Folder, FileText, Settings, Loader2, FolderOpen, Sparkles, Filter, Pause, Play, MessageSquare } from "@lucide/vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useAppStore, type EmbedProgress } from "./stores/app";
import { useSearchStore } from "./stores/search";
import { useFilesStore } from "./stores/files";
import { useTagsStore } from "./stores/tags";
import SearchPalette from "./components/SearchPalette.vue";
import FileList from "./components/FileList.vue";
import DetailPanel from "./components/DetailPanel.vue";
import ToastHost from "./components/ToastHost.vue";
import SettingsModal from "./components/SettingsModal.vue";
import ChatPanel from "./components/ChatPanel.vue";
import { useChatStore } from "./stores/chat";

const app = useAppStore();
const search = useSearchStore();
const files = useFilesStore();
const tags = useTagsStore();
const chat = useChatStore();
const indexing = ref(false);
const paused = ref(false);
const showKindMenu = ref(false);
const showSettings = ref(false);

async function togglePause() {
  if (paused.value) {
    await invoke("resume_indexing");
    paused.value = false;
  } else if (indexing.value) {
    await invoke("pause_indexing");
    paused.value = true;
  }
}

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

const embeddingPct = computed(() => {
  const p = app.embedding;
  if (p.total === 0) return 0;
  return Math.min(100, Math.round((p.embedded / p.total) * 100));
});

const kindOptions: { value: string | null; label: string }[] = [
  { value: null, label: "全部" },
  { value: "pdf", label: "PDF" },
  { value: "markdown", label: "Markdown" },
  { value: "text", label: "文本" },
  { value: "code", label: "代码" },
  { value: "image", label: "图片" },
  { value: "document", label: "文档" },
  { value: "spreadsheet", label: "表格" },
  { value: "archive", label: "压缩包" },
];

const currentKindLabel = computed(() => {
  return kindOptions.find((o) => o.value === files.kindFilter)?.label || "全部";
});

onMounted(async () => {
  await app.bootstrap();
  app.applyTheme(app.theme);
  if (app.indexedFolders.length > 0) {
    await files.reload();
  }
  files.loadFavoriteFiles();
  tags.loadTags();
  await listen<{ total: number; duration_ms: number }>("index-complete", async () => {
    indexing.value = false;
    await app.refreshStats();
    await files.reload();
  });
  await listen<{ current: number; total: number; phase: string }>("index-progress", (e) => {
    if (e.payload.phase === "Indexing") indexing.value = true;
    if (e.payload.phase === "Completed" || e.payload.phase === "Cancelled") indexing.value = false;
  });
  await listen<EmbedProgress>("embedding-progress", (e) => {
    app.updateEmbedding(e.payload);
    if (e.payload.phase === "Completed") {
      app.refreshStats();
    }
  });
});

watch(
  () => app.stats.files,
  async (n, old) => {
    if (n !== old && n > 0) await files.reload();
  },
);
</script>

<template>
  <div class="flex h-screen w-screen flex-col bg-background text-foreground">
    <header class="flex h-12 items-center gap-3 border-b border-border px-4" data-tauri-drag-region>
      <span class="text-sm font-semibold tracking-tight">Glean</span>
      <button
        class="ml-4 flex flex-1 items-center gap-2 rounded-md bg-muted px-3 py-1.5 text-left text-sm text-muted-foreground transition hover:bg-muted/80"
        @click="search.paletteOpen = true"
      >
        <Search class="size-4" />
        <span class="flex-1">搜索文件、内容、命令...</span>
        <kbd class="text-xs">⌘K</kbd>
      </button>
      <button
        class="rounded-md p-1.5 text-muted-foreground hover:bg-muted"
        aria-label="AI 助手"
        title="AI 助手"
        @click="chat.togglePanel()"
      >
        <MessageSquare class="size-4" />
      </button>
      <button
        class="rounded-md p-1.5 text-muted-foreground hover:bg-muted"
        aria-label="设置"
        @click="showSettings = true"
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

      <main class="flex flex-1 flex-col overflow-hidden">
        <div
          v-if="!app.indexedFolders?.length"
          class="flex flex-1 flex-col items-center justify-center text-muted-foreground"
        >
          <p class="text-sm">还没有索引任何文件夹</p>
          <button
            :disabled="indexing"
            class="mt-3 flex items-center gap-2 rounded-md bg-primary px-4 py-1.5 text-sm text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
            @click="pickAndIndex"
          >
            <FolderOpen class="size-4" />
            选择文件夹开始索引
          </button>
        </div>
        <template v-else>
          <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-2.5">
            <div class="flex flex-1 items-center gap-2 rounded-md bg-muted/50 px-2 py-1">
              <Search class="size-3.5 text-muted-foreground" />
              <input
                :value="files.nameFilter"
                placeholder="过滤当前列表..."
                class="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
                @input="files.setNameFilter(($event.target as HTMLInputElement).value)"
              />
              <span class="text-xs text-muted-foreground">
                {{ files.filtered.length }}/{{ files.items.length }}
              </span>
            </div>
            <div class="flex items-center gap-2">
              <div class="relative">
                <button
                  class="flex items-center gap-1 rounded-md bg-muted px-2 py-1 text-xs hover:bg-muted/80"
                  @click="showKindMenu = !showKindMenu"
                >
                  <Filter class="size-3" />
                  {{ currentKindLabel }}
                </button>
                <div
                  v-if="showKindMenu"
                  class="absolute right-0 top-full z-10 mt-1 w-32 rounded-md border border-border bg-background py-1 shadow-lg"
                  @mouseleave="showKindMenu = false"
                >
                  <button
                    v-for="opt in kindOptions"
                    :key="String(opt.value)"
                    :class="[
                      'block w-full px-3 py-1 text-left text-xs hover:bg-muted',
                      files.kindFilter === opt.value ? 'font-medium text-foreground' : '',
                    ]"
                    @click="files.setKindFilter(opt.value); showKindMenu = false"
                  >
                    {{ opt.label }}
                  </button>
                </div>
              </div>
              <button
                v-if="indexing"
                class="flex items-center gap-1 rounded-md bg-muted px-2 py-1 text-xs hover:bg-muted/80"
                :title="paused ? '恢复索引' : '暂停索引'"
                @click="togglePause"
              >
                <Play v-if="paused" class="size-3" />
                <Pause v-else class="size-3" />
                {{ paused ? "继续" : "暂停" }}
              </button>
              <div
                v-if="app.embedding.phase === 'Downloading'"
                class="group relative flex items-center gap-1.5 rounded-md bg-blue-500/10 px-2 py-1 text-[11px] text-blue-600 dark:text-blue-400"
              >
                <Loader2 class="size-3 animate-spin" />
                下载模型
                <div
                  class="pointer-events-none absolute right-0 top-full z-50 mt-2 hidden w-64 rounded-md border border-border bg-background p-3 text-xs text-foreground shadow-lg group-hover:block"
                >
                  <div class="mb-2 font-medium">下载 Embedding 模型</div>
                  <div class="mb-2 text-muted-foreground">
                    BAAI/bge-small-en-v1.5（约 130MB），从 HuggingFace 下载到本地缓存。
                  </div>
                  <div class="mb-2 h-1 w-full overflow-hidden rounded-full bg-muted">
                    <div class="h-full w-1/3 animate-[indeterminate_1.2s_ease-in-out_infinite] rounded-full bg-blue-500"></div>
                  </div>
                  <div class="text-[10px] text-muted-foreground">
                    用途：把文本转成向量，支持"按语义搜索"<br />
                    位置：<code class="break-all">~/.cache/huggingface/hub/</code><br />
                    只在首次启动下载一次
                  </div>
                </div>
              </div>
              <div
                v-else-if="app.embedding.phase === 'Embedding'"
                class="flex items-center gap-1.5 rounded-md bg-primary/10 px-2 py-1 text-[11px] text-primary"
                :title="`已向量化 ${app.embedding.embedded} / ${app.embedding.total} chunk`"
              >
                <Sparkles class="size-3 animate-pulse" />
                向量化 {{ embeddingPct }}%
              </div>
              <div
                v-else-if="app.embedding.phase === 'Completed'"
                class="flex items-center gap-1.5 rounded-md bg-emerald-500/10 px-2 py-1 text-[11px] text-emerald-600 dark:text-emerald-400"
                :title="`共 ${app.stats.chunks} chunks 已向量化`"
              >
                <Sparkles class="size-3" />
                向量就绪
              </div>
              <div
                v-else-if="app.embedding.phase === 'Failed'"
                class="flex items-center gap-1.5 rounded-md bg-red-500/10 px-2 py-1 text-[11px] text-red-600 dark:text-red-400"
                title="向量化失败，请查看日志"
              >
                <Sparkles class="size-3" />
                向量化失败
              </div>
              <button
                class="flex items-center gap-1.5 rounded-md bg-muted px-2.5 py-1 text-xs transition hover:bg-muted/80 disabled:opacity-50"
                @click="pickAndIndex"
              >
                <Loader2 v-if="indexing && !paused" class="size-3 animate-spin" />
                <FolderOpen v-else class="size-3" />
                添加
              </button>
            </div>
          </div>

          <div class="flex flex-1 overflow-hidden">
            <div class="flex-1 overflow-hidden">
              <FileList />
            </div>
            <DetailPanel />
            <ChatPanel />
          </div>
        </template>
      </main>
    </div>

    <SearchPalette />
    <ToastHost />
    <SettingsModal v-if="showSettings" @close="showSettings = false" />
  </div>
</template>
