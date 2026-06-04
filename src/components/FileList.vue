<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { invoke } from "@tauri-apps/api/core";
import { useFilesStore, type FileEntry } from "../stores/files";
import { useToastStore } from "../stores/toast";
import { Loader2, ExternalLink, FolderOpen, Copy, Search, Star } from "@lucide/vue";
import { kindIcon, formatSize, formatDate } from "../utils/fileKind";
import ContextMenu from "./ContextMenu.vue";

const { t } = useI18n();
const store = useFilesStore();
const toast = useToastStore();
const containerRef = useTemplateRef<HTMLDivElement>("containerRef");
const hoverId = ref<number | null>(null);
const ctxMenu = ref<{ x: number; y: number; fileId: number } | null>(null);

const rowHeight = 40;

const virtualizer = useVirtualizer(
  computed(() => ({
    count: store.filtered.length,
    getScrollElement: () => containerRef.value,
    estimateSize: () => rowHeight,
    overscan: 20,
  })),
);

const items = computed(() => virtualizer.value.getVirtualItems());

function isActive(item: FileEntry) {
  return item.id === store.selectedId || item.id === hoverId.value;
}

function isSelected(item: FileEntry) {
  return item.id === store.selectedId;
}

function onDoubleClick(item: FileEntry) {
  invoke("open_file", { path: item.path });
}

async function openFile(path: string) {
  await invoke("open_file", { path });
}

async function revealInFinder(path: string) {
  await invoke("reveal_in_finder", { path });
}

async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    toast.push("已复制路径", "success");
  } catch {
    toast.push("复制失败", "error");
  }
}

function onContextMenu(e: MouseEvent, item: FileEntry) {
  e.preventDefault();
  store.select(item.id);
  ctxMenu.value = { x: e.clientX, y: e.clientY, fileId: item.id };
}

function closeCtxMenu() {
  ctxMenu.value = null;
}

function ctxFile(): FileEntry | null {
  if (!ctxMenu.value) return null;
  return store.items.find((f) => f.id === ctxMenu.value!.fileId) || null;
}

async function ctxAction(action: "open" | "finder" | "copy") {
  const f = ctxFile();
  if (!f) return;
  closeCtxMenu();
  if (action === "open") await openFile(f.path);
  else if (action === "finder") await revealInFinder(f.path);
  else if (action === "copy") await copyPath(f.path);
}

async function doToggleFavorite(fileId: number) {
  await store.toggleFavorite(fileId);
}

watch(() => store.filtered, () => virtualizer.value.scrollToIndex(0), { flush: "post" });
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="flex items-center gap-0.5 border-b border-border bg-muted/20 px-2 py-1 text-xs text-muted-foreground">
      <button
        :class="[
          'rounded-md px-2.5 py-1 text-xs font-medium transition-colors',
          !store.showRecent && !store.showFavorites
            ? 'bg-background text-foreground shadow-xs'
            : 'text-muted-foreground/70 hover:text-foreground',
        ]"
        @click="store.setViewMode('all')"
      >
        {{ t('filelist.all') }}
      </button>
      <button
        :class="[
          'rounded-md px-2.5 py-1 text-xs font-medium transition-colors',
          store.showRecent
            ? 'bg-background text-foreground shadow-xs'
            : 'text-muted-foreground/70 hover:text-foreground',
        ]"
        @click="store.toggleRecent()"
      >
        {{ t('filelist.recent') }}
      </button>
      <button
        :class="[
          'rounded-md px-2.5 py-1 text-xs font-medium transition-colors',
          store.showFavorites
            ? 'bg-background text-foreground shadow-xs'
            : 'text-muted-foreground/70 hover:text-foreground',
        ]"
        @click="store.toggleFavorites()"
      >
        {{ t('filelist.favorites') }}
      </button>
    </div>
    <div
      class="grid grid-cols-[1fr_70px_65px_110px] gap-3 border-b border-border bg-muted/15 px-3 py-2 text-[11px] font-medium text-muted-foreground/60"
    >
      <button
        class="flex items-center gap-1 text-left transition-colors hover:text-foreground/80"
        @click="store.toggleSort('name')"
      >
        {{ t('filelist.name') }}
        <span v-if="store.sortKey === 'name'" class="text-foreground/60 text-[10px]">
          {{ store.sortDir === "asc" ? "▲" : "▼" }}
        </span>
      </button>
      <button
        class="flex items-center gap-1 transition-colors hover:text-foreground/80"
        @click="store.toggleSort('ext')"
      >
        {{ t('filelist.type') }}
        <span v-if="store.sortKey === 'ext'" class="text-foreground/60 text-[10px]">
          {{ store.sortDir === "asc" ? "▲" : "▼" }}
        </span>
      </button>
      <button
        class="flex items-center gap-1 transition-colors hover:text-foreground/80"
        @click="store.toggleSort('size')"
      >
        {{ t('filelist.size') }}
        <span v-if="store.sortKey === 'size'" class="text-foreground/60 text-[10px]">
          {{ store.sortDir === "asc" ? "▲" : "▼" }}
        </span>
      </button>
      <button
        class="flex items-center gap-1 transition-colors hover:text-foreground/80"
        @click="store.toggleSort('mtime')"
      >
        {{ t('filelist.mtime') }}
        <span v-if="store.sortKey === 'mtime'" class="text-foreground/60 text-[10px]">
          {{ store.sortDir === "asc" ? "▲" : "▼" }}
        </span>
      </button>
    </div>

    <div ref="containerRef" class="flex-1 overflow-auto">
      <div
        v-if="store.loading && !store.items.length"
        class="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground"
      >
        <Loader2 class="size-5 animate-spin text-muted-foreground/50" />
        <span class="text-xs">{{ t('filelist.preview_loading') }}</span>
      </div>
      <div v-else-if="store.error" class="px-3 py-6 text-sm text-red-500">
        {{ store.error }}
      </div>
      <div
        v-else-if="!store.filtered.length"
        class="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground"
      >
        <Search v-if="store.nameFilter" class="size-6 opacity-40" />
        <span class="text-sm">
          {{ store.showFavorites ? t('filelist.empty_favorites') : store.showRecent ? t('filelist.empty_recent') : store.nameFilter ? t('filelist.empty_filter', { query: store.nameFilter }) : t('filelist.empty_all') }}
        </span>
        <span v-if="store.nameFilter" class="text-xs text-muted-foreground/60">{{ t('filelist.empty_filter_hint') }}</span>
      </div>
      <div
        v-else
        :style="{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }"
      >
        <div
          v-for="vi in items"
          :key="store.filtered[vi.index]!.id"
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: `${vi.size}px`,
            transform: `translateY(${vi.start}px)`,
          }"
        >
          <div
            :class="[
              'grid h-full grid-cols-[1fr_70px_65px_110px] items-center gap-3 px-3 text-[13px] cursor-default transition-colors',
              isSelected(store.filtered[vi.index]!)
                ? 'bg-accent/12 text-foreground'
                : vi.index % 2 === 1
                  ? 'bg-muted/15'
                  : '',
              !isSelected(store.filtered[vi.index]!) && hoverId === store.filtered[vi.index]!.id
                ? 'bg-muted/40'
                : '',
            ]"
            @click="store.select(store.filtered[vi.index]!.id)"
            @dblclick="onDoubleClick(store.filtered[vi.index]!)"
            @mouseenter="hoverId = store.filtered[vi.index]!.id"
            @mouseleave="hoverId = null"
            @contextmenu="onContextMenu($event, store.filtered[vi.index]!)"
          >
            <div class="flex min-w-0 items-center gap-2">
              <component
                :is="kindIcon(store.filtered[vi.index]!.kind)"
                :class="[
                  'size-4 shrink-0',
                  isSelected(store.filtered[vi.index]!) ? 'text-accent' : 'text-muted-foreground/60',
                ]"
              />
              <span
                :class="[
                  'truncate',
                  isSelected(store.filtered[vi.index]!) ? 'font-medium' : '',
                ]"
              >{{ store.filtered[vi.index]!.name }}</span>
              <button
                class="shrink-0 rounded p-0.5 opacity-0 hover:opacity-100 transition-opacity hover:bg-muted/60"
                :class="{ 'opacity-100': store.favoriteIds.has(store.filtered[vi.index]!.id) }"
                :title="store.favoriteIds.has(store.filtered[vi.index]!.id) ? t('filelist.rm_fav') : t('filelist.add_fav')"
                @click.stop="doToggleFavorite(store.filtered[vi.index]!.id)"
              >
                <Star
                  class="size-3"
                  :class="store.favoriteIds.has(store.filtered[vi.index]!.id)
                    ? 'fill-yellow-400 text-yellow-400'
                    : 'text-muted-foreground'"
                />
              </button>
            </div>
            <span class="text-xs text-muted-foreground/70">
              .{{ store.filtered[vi.index]!.ext || "—" }}
            </span>
            <span class="text-xs text-muted-foreground/70">
              {{ formatSize(store.filtered[vi.index]!.size) }}
            </span>
            <span class="text-xs text-muted-foreground/70">
              {{ formatDate(store.filtered[vi.index]!.mtime) }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="store.items.length"
      class="flex items-center gap-2 border-t border-border bg-muted/20 px-3 py-1 text-[10px] text-muted-foreground/60"
    >
      <template v-if="store.showFavorites">
        <span>{{ store.filtered.length }} {{ t('filelist.selected_favorites') }}</span>
      </template>
      <template v-else-if="store.showRecent">
        <span>{{ store.filtered.length }} {{ t('filelist.selected_recent') }}</span>
      </template>
      <template v-else-if="store.nameFilter">
        <span>{{ t('filelist.selected_filter', { count: store.filtered.length, total: store.items.length }) }}</span>
      </template>
      <template v-else>
        <span>{{ t('filelist.status_total', { count: store.items.length }) }}</span>
        <span class="text-muted-foreground/30">·</span>
        <span>{{ t('filelist.status_hint') }}</span>
      </template>
    </div>

    <ContextMenu
      v-if="ctxMenu"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      @close="closeCtxMenu"
    >
      <button
        class="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-muted"
        @click="ctxAction('open')"
      >
        <ExternalLink class="size-3.5" />
        {{ t('filelist.open') }}
      </button>
      <button
        class="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-muted"
        @click="ctxAction('finder')"
      >
        <FolderOpen class="size-3.5" />
        {{ t('filelist.reveal') }}
      </button>
      <button
        class="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-muted"
        @click="ctxAction('copy')"
      >
        <Copy class="size-3.5" />
        {{ t('filelist.copy_path') }}
      </button>
    </ContextMenu>
  </div>
</template>
