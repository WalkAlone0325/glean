<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
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
import OnboardingWizard from "./components/OnboardingWizard.vue";
import { useChatStore } from "./stores/chat";

const app = useAppStore();
const search = useSearchStore();
const files = useFilesStore();
const tags = useTagsStore();
const { locale: i18nLocale, t } = useI18n();
const chat = useChatStore();
const indexing = ref(false);
const paused = ref(false);
const showKindMenu = ref(false);
const showSettings = ref(false);
const showFirstRun = ref(true);

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

const kindOptions = computed(() => [
  { value: null, label: t('kind.all') },
  { value: "pdf", label: t('kind.pdf') },
  { value: "markdown", label: t('kind.markdown') },
  { value: "text", label: t('kind.text') },
  { value: "code", label: t('kind.code') },
  { value: "image", label: t('kind.image') },
  { value: "document", label: t('kind.document') },
  { value: "spreadsheet", label: t('kind.spreadsheet') },
  { value: "archive", label: t('kind.archive') },
]);

const currentKindLabel = computed(() => {
  return kindOptions.value.find((o) => o.value === files.kindFilter)?.label || t('kind.all');
});

onMounted(async () => {
  await app.bootstrap();
  app.applyTheme(app.theme);
  i18nLocale.value = app.locale;
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

watch(() => app.locale, (loc) => {
  i18nLocale.value = loc;
});
</script>

<template>
    <div class="flex h-screen w-screen flex-col bg-background text-foreground">
      <header class="flex h-12 shrink-0 items-center gap-3 border-b border-border bg-muted/20 px-4" data-tauri-drag-region>
        <span class="flex items-center gap-1.5 text-sm font-bold tracking-tight">
          <Sparkles class="size-4 text-primary" />
          Glean
        </span>
        <button
          class="group ml-4 flex flex-1 items-center gap-2 rounded-lg border border-border bg-background px-3 py-1.5 text-left text-sm text-muted-foreground shadow-sm transition hover:border-muted-foreground/30 hover:bg-muted/50"
          @click="search.paletteOpen = true"
        >
          <Search class="size-4" />
          <span class="flex-1">{{ t('header.search_placeholder') }}</span>
          <kbd class="rounded-md bg-muted px-1.5 py-0.5 text-[10px] font-medium tracking-wider">⌘K</kbd>
        </button>
        <button
          class="rounded-lg p-1.5 text-muted-foreground hover:bg-muted transition-colors"
          :aria-label="t('header.ai_assistant')"
          :title="t('header.ai_assistant')"
          @click="chat.togglePanel()"
        >
          <MessageSquare class="size-4" />
        </button>
        <button
          class="rounded-lg p-1.5 text-muted-foreground hover:bg-muted transition-colors"
          :aria-label="t('header.settings')"
          @click="showSettings = true"
        >
          <Settings class="size-4" />
        </button>
      </header>

      <div class="flex flex-1 overflow-hidden">
        <aside class="flex w-52 shrink-0 flex-col border-r border-border bg-muted/20">
          <nav class="space-y-0.5 p-2 pb-3 text-sm">
            <a class="flex items-center gap-2 rounded-lg bg-primary/10 px-2.5 py-1.5 font-medium text-primary">
              <Folder class="size-4" />
              {{ t('sidebar.all_files') }}
              <span class="ml-auto text-xs tabular-nums text-primary/70">{{ app.stats.files }}</span>
            </a>
            <a class="flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground">
              <FileText class="size-4" />
              {{ t('sidebar.recent') }}
            </a>
          </nav>
          <div class="mx-3 border-t border-border" />
          <div class="flex-1 overflow-auto p-2 pt-3">
            <div class="mb-2 px-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/70">
              {{ t('sidebar.indexed_folders') }}
            </div>
            <div v-if="!app.indexedFolders?.length" class="px-2 text-xs text-muted-foreground">
              {{ t('sidebar.no_folders') }}
            </div>
            <ul v-else class="space-y-0.5 text-xs">
              <li
                v-for="f in app.indexedFolders"
                :key="f"
                class="flex items-center gap-1.5 truncate rounded-md px-2 py-1 text-muted-foreground transition-colors hover:bg-muted"
                :title="f"
              >
                <FolderOpen class="size-3.5 shrink-0" />
                <span class="truncate">{{ f }}</span>
              </li>
            </ul>
          </div>
        </aside>

        <main class="flex flex-1 flex-col overflow-hidden">
          <div
            v-if="!app.indexedFolders?.length"
            class="flex flex-1 flex-col items-center justify-center gap-4 text-muted-foreground"
          >
            <div class="rounded-2xl bg-muted/40 p-5">
              <Search class="size-10 text-muted-foreground/40" />
            </div>
            <div class="text-center">
              <p class="text-sm font-medium">{{ t('empty_state.title') }}</p>
              <p class="mt-1 text-xs opacity-60">{{ t('empty_state.subtitle') }}</p>
            </div>
            <button
              :disabled="indexing"
              class="flex items-center gap-2 rounded-lg bg-primary px-5 py-2 text-sm font-medium text-primary-foreground shadow-sm transition hover:opacity-90 disabled:opacity-50"
              @click="pickAndIndex"
            >
              <FolderOpen class="size-4" />
              {{ t('empty_state.button') }}
            </button>
          </div>
        <template v-else>
          <div class="flex items-center gap-3 border-b border-border px-4 py-2">
            <div class="flex flex-1 items-center gap-2 rounded-lg border border-border/60 bg-background px-2.5 py-1.5 text-sm shadow-sm transition-colors focus-within:border-muted-foreground/30">
              <Search class="size-3.5 text-muted-foreground/60" />
              <input
                :value="files.nameFilter"
                :placeholder="t('toolbar.filter_placeholder')"
                class="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/50"
                @input="files.setNameFilter(($event.target as HTMLInputElement).value)"
              />
              <span class="text-[11px] tabular-nums text-muted-foreground/50">
                {{ files.filtered.length }}/{{ files.items.length }}
              </span>
            </div>
            <div class="flex items-center gap-1.5">
              <div class="relative">
                <button
                  class="flex items-center gap-1 rounded-lg border border-border/60 bg-background px-2.5 py-1.5 text-xs font-medium text-muted-foreground shadow-sm transition-colors hover:bg-muted/50"
                  @click="showKindMenu = !showKindMenu"
                >
                  <Filter class="size-3" />
                  {{ currentKindLabel }}
                </button>
                <div
                  v-if="showKindMenu"
                  class="absolute right-0 top-full z-10 mt-1 w-32 rounded-lg border border-border bg-background py-1 shadow-lg"
                  @mouseleave="showKindMenu = false"
                >
                  <button
                    v-for="opt in kindOptions"
                    :key="String(opt.value)"
                    :class="[
                      'block w-full px-3 py-1.5 text-left text-xs transition-colors hover:bg-muted',
                      files.kindFilter === opt.value ? 'font-medium text-foreground' : 'text-muted-foreground',
                    ]"
                    @click="files.setKindFilter(opt.value); showKindMenu = false"
                  >
                    {{ opt.label }}
                  </button>
                </div>
              </div>
              <button
                v-if="indexing"
                class="flex items-center gap-1 rounded-lg border border-border/60 bg-background px-2.5 py-1.5 text-xs font-medium text-muted-foreground shadow-sm transition-colors hover:bg-muted/50"
                :title="paused ? t('toolbar.resume') : t('toolbar.pause')"
                @click="togglePause"
              >
                <Play v-if="paused" class="size-3" />
                <Pause v-else class="size-3" />
                {{ paused ? t('toolbar.resume') : t('toolbar.pause') }}
              </button>
              <div
                v-if="app.embedding.phase === 'Downloading'"
                class="group relative flex items-center gap-1.5 rounded-lg bg-blue-500/10 px-2.5 py-1.5 text-[11px] font-medium text-blue-600 dark:text-blue-400"
              >
                <Loader2 class="size-3 animate-spin" />
                {{ t('toolbar.downloading_model') }}
                <div
                  class="pointer-events-none absolute right-0 top-full z-50 mt-2 hidden w-64 rounded-lg border border-border bg-background p-3 text-xs text-foreground shadow-lg group-hover:block"
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
                class="flex items-center gap-1.5 rounded-lg bg-primary/10 px-2.5 py-1.5 text-[11px] font-medium text-primary"
                :title="`已向量化 ${app.embedding.embedded} / ${app.embedding.total} chunk`"
              >
                <Sparkles class="size-3 animate-pulse" />
                {{ t('toolbar.embedding_pct', { pct: embeddingPct }) }}
              </div>
              <div
                v-else-if="app.embedding.phase === 'Completed'"
                class="flex items-center gap-1.5 rounded-lg bg-emerald-500/10 px-2.5 py-1.5 text-[11px] font-medium text-emerald-600 dark:text-emerald-400"
                :title="t('toolbar.embedding_ready')"
              >
                <Sparkles class="size-3" />
                {{ t('toolbar.embedding_ready') }}
              </div>
              <div
                v-else-if="app.embedding.phase === 'Failed'"
                class="flex items-center gap-1.5 rounded-lg bg-red-500/10 px-2.5 py-1.5 text-[11px] font-medium text-red-600 dark:text-red-400"
                :title="t('toolbar.embedding_failed')"
              >
                <Sparkles class="size-3" />
                {{ t('toolbar.embedding_failed') }}
              </div>
              <div class="mx-1 h-5 w-px bg-border" />
              <button
                class="flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground shadow-sm transition hover:opacity-90 disabled:opacity-50"
                @click="pickAndIndex"
              >
                <Loader2 v-if="indexing && !paused" class="size-3 animate-spin" />
                <FolderOpen v-else class="size-3" />
                {{ t('toolbar.add_folder') }}
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
    <OnboardingWizard v-if="app.ready && app.isFirstRun" @close="showFirstRun = false" />
  </div>
</template>
