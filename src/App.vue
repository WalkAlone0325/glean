<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Search, Folder, FileText, Settings, Loader2, FolderOpen, Sparkles, Filter, Pause, Play, MessageSquare, Star, PanelLeftClose, PanelLeftOpen, Tags, ChevronDown, ChevronRight } from "@lucide/vue";
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
const sidebarCollapsed = ref(false);
const detailCollapsed = ref(false);
const tagsExpanded = ref(false);
const selectedTag = ref<string | null>(null);

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

watch(() => files.selectedId, (id) => {
  if (id !== null) detailCollapsed.value = false;
});

function selectTag(tagName: string | null) {
  selectedTag.value = tagName;
  files.setTagFilter(tagName);
  if (tagName) {
    files.showRecent = false;
    files.showFavorites = false;
  }
}
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
        <span class="flex-1">{{ t('header.search_placeholder') }}</span>
        <kbd class="text-xs">⌘K</kbd>
      </button>
      <button
        class="rounded-md p-1.5 text-muted-foreground hover:bg-muted"
        :aria-label="t('header.ai_assistant')"
        :title="t('header.ai_assistant')"
        @click="chat.togglePanel()"
      >
        <MessageSquare class="size-4" />
      </button>
      <button
        class="rounded-md p-1.5 text-muted-foreground hover:bg-muted"
        :aria-label="t('header.settings')"
        @click="showSettings = true"
      >
        <Settings class="size-4" />
      </button>
    </header>

    <div class="flex flex-1 overflow-hidden">
      <aside :class="['border-r border-border bg-muted/30 transition-all', sidebarCollapsed ? 'w-0 overflow-hidden' : 'w-56']">
        <div v-if="!sidebarCollapsed" class="flex h-full flex-col p-3">
          <div class="flex-1 space-y-6 overflow-auto">
            <nav class="space-y-1 text-sm">
              <a
                :class="[
                  'flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5',
                  !files.showRecent && !files.showFavorites
                    ? 'bg-muted'
                    : 'hover:bg-muted',
                ]"
                @click="files.setViewMode('all'); selectTag(null)"
              >
                <Folder class="size-4" />
                {{ t('sidebar.all_files') }}
                <span class="ml-auto text-xs text-muted-foreground">{{ selectedTag ? files.tagFilteredItems.length : app.stats.files }}</span>
              </a>
              <a
                :class="[
                  'flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5',
                  files.showRecent ? 'bg-muted' : 'hover:bg-muted',
                ]"
                @click="files.toggleRecent()"
              >
                <FileText class="size-4" />
                {{ t('sidebar.recent') }}
              </a>
              <a
                :class="[
                  'flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5',
                  files.showFavorites ? 'bg-muted' : 'hover:bg-muted',
                ]"
                @click="files.toggleFavorites()"
              >
                <Star class="size-4" />
                {{ t('sidebar.favorites') }}
                <span v-if="files.favoriteIds.size" class="ml-auto text-xs text-muted-foreground">{{ files.favoriteIds.size }}</span>
              </a>
              <div class="pt-2">
                <a
                  class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-xs text-muted-foreground hover:bg-muted"
                  @click="tagsExpanded = !tagsExpanded"
                >
                  <ChevronRight v-if="!tagsExpanded" class="size-3" />
                  <ChevronDown v-else class="size-3" />
                  <Tags class="size-4" />
                  {{ t('sidebar.tags') }}
                </a>
                <div v-if="tagsExpanded" class="ml-3 mt-1 space-y-0.5">
                  <div v-if="!tags.all.length" class="px-2 py-1 text-xs text-muted-foreground">
                    {{ t('sidebar.no_tags') }}
                  </div>
                  <a
                    v-for="tag in tags.all"
                    :key="tag.id"
                    :class="[
                      'flex cursor-pointer items-center gap-2 rounded-md px-2 py-1 text-sm',
                      selectedTag === tag.name ? 'bg-muted font-medium' : 'hover:bg-muted',
                    ]"
                    @click="selectTag(tag.name)"
                  >
                    <span
                      class="inline-block size-2 rounded-full"
                      :style="{ backgroundColor: tag.color || '#888' }"
                    />
                    {{ tag.name }}
                    <span class="ml-auto text-muted-foreground">{{ tag.file_count }}</span>
                  </a>
                  <a
                    v-if="selectedTag"
                    class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1 text-sm text-muted-foreground hover:bg-muted"
                    @click="selectTag(null)"
                  >
                    {{ t('kind.all') }}
                  </a>
                </div>
              </div>
            </nav>
            <div>
              <div class="mb-2 px-2 text-xs uppercase tracking-wide text-muted-foreground">
                {{ t('sidebar.indexed_folders') }}
              </div>
              <div v-if="!app.indexedFolders?.length" class="px-2 text-xs text-muted-foreground">
                {{ t('sidebar.no_folders') }}
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
          </div>
          <button
            class="mt-auto flex w-full items-center gap-2 rounded-md px-2 py-2 text-sm text-muted-foreground hover:bg-muted transition-colors"
            :title="t('sidebar.collapse')"
            @click="sidebarCollapsed = true"
          >
            <PanelLeftClose class="size-5 shrink-0" />
            <span class="text-xs">{{ t('sidebar.collapse') }}</span>
          </button>
        </div>
      </aside>
      <div
        v-if="sidebarCollapsed"
        class="flex w-7 shrink-0 cursor-pointer items-end justify-center border-r border-border bg-muted/30 pb-3 transition-colors hover:bg-muted/50"
        :title="t('sidebar.expand')"
        @click="sidebarCollapsed = false"
      >
        <PanelLeftOpen class="size-5 text-muted-foreground" />
      </div>

      <main class="flex flex-1 flex-col overflow-hidden">
        <div
          v-if="!app.indexedFolders?.length"
          class="flex flex-1 flex-col items-center justify-center text-muted-foreground"
        >
          <p class="text-sm">{{ t('empty_state.title') }}</p>
          <button
            :disabled="indexing"
            class="mt-3 flex items-center gap-2 rounded-md bg-primary px-4 py-1.5 text-sm text-primary-foreground transition hover:opacity-90 disabled:opacity-50"
            @click="pickAndIndex"
          >
            <FolderOpen class="size-4" />
            {{ t('empty_state.button') }}
          </button>
        </div>
        <template v-else>
          <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-2.5">
            <div class="flex flex-1 items-center gap-2 rounded-md bg-muted/50 px-2 py-1">
              <Search class="size-3.5 text-muted-foreground" />
              <input
                :value="files.nameFilter"
                :placeholder="t('toolbar.filter_placeholder')"
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
                :title="paused ? t('toolbar.resume') : t('toolbar.pause')"
                @click="togglePause"
              >
                <Play v-if="paused" class="size-3" />
                <Pause v-else class="size-3" />
                {{ paused ? t('toolbar.resume') : t('toolbar.pause') }}
              </button>
              <div
                v-if="app.embedding.phase === 'Downloading'"
                class="group relative flex items-center gap-1.5 rounded-md bg-blue-500/10 px-2 py-1 text-[11px] text-blue-600 dark:text-blue-400"
              >
                <Loader2 class="size-3 animate-spin" />
                {{ t('toolbar.downloading_model') }}
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
                {{ t('toolbar.embedding_pct', { pct: embeddingPct }) }}
              </div>
              <div
                v-else-if="app.embedding.phase === 'Completed'"
                class="flex items-center gap-1.5 rounded-md bg-emerald-500/10 px-2 py-1 text-[11px] text-emerald-600 dark:text-emerald-400"
                :title="`共 ${app.stats.chunks} chunks 已向量化`"
              >
                <Sparkles class="size-3" />
                {{ t('toolbar.embedding_ready') }}
              </div>
              <div
                v-else-if="app.embedding.phase === 'Failed'"
                class="flex items-center gap-1.5 rounded-md bg-red-500/10 px-2 py-1 text-[11px] text-red-600 dark:text-red-400"
                title="向量化失败，请查看日志"
              >
                <Sparkles class="size-3" />
                {{ t('toolbar.embedding_failed') }}
              </div>
              <button
                class="flex items-center gap-1.5 rounded-md bg-muted px-2.5 py-1 text-xs transition hover:bg-muted/80 disabled:opacity-50"
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
            <DetailPanel v-if="!detailCollapsed" @close="detailCollapsed = true" />
            <ChatPanel />
            <div
              v-if="detailCollapsed && !chat.panelOpen"
              class="flex w-7 shrink-0 cursor-pointer items-end justify-center border-l border-border bg-muted/30 pb-3 transition-colors hover:bg-muted/50"
              :title="t('detail.collapse')"
              @click="detailCollapsed = false"
            >
              <PanelLeftOpen class="size-5 text-muted-foreground rotate-180" />
            </div>
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
