<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { invoke } from "@tauri-apps/api/core";
import { useFilesStore, type FileEntry } from "../stores/files";
import { useToastStore } from "../stores/toast";
import { Loader2, ExternalLink, FolderOpen, Copy, Search, Star } from "@lucide/vue";
import { kindIcon, formatSize, formatDate } from "../utils/fileKind";
import ContextMenu from "./ContextMenu.vue";

const store = useFilesStore();
const toast = useToastStore();
const containerRef = useTemplateRef<HTMLDivElement>("containerRef");
const hoverId = ref<number | null>(null);
const ctxMenu = ref<{ x: number; y: number; fileId: number } | null>(null);

const rowHeight = 44;

const virtualizer = useVirtualizer(
  computed(() => ({
    count: store.filtered.length,
    getScrollElement: () => containerRef.value,
    estimateSize: () => rowHeight,
    overscan: 16,
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
    <div class="flex items-center gap-0.5 border-b border-border bg-muted/10 px-3 py-1.5 text-xs text-muted-foreground">
      <button
        :class="[
          'rounded-md px-2.5 py-1 text-xs font-medium transition-colors',
          !store.showRecent && !store.showFavorites
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground',
        ]"
        @click="store.setViewMode('all')"
      >
        全部文件
      </button>
      <button
        :class="[
          'rounded-md px-2.5 py-1 text-xs font-medium transition-colors',
          store.showRecent
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground',
        ]"
        @click="store.toggleRecent()"
      >
        最近查看
      </button>
      <button
        :class="[
          'rounded-md px-2.5 py-1 text-xs font-medium transition-colors',
          store.showFavorites
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground',
        ]"
        @click="store.toggleFavorites()"
      >
        星标文件
      </button>
    </div>
    <div
      class="grid grid-cols-[1fr_80px_70px_120px] gap-3 border-b border-border bg-muted/10 px-3 py-2 text-xs font-medium text-muted-foreground/80"
    >
      <button
        class="flex items-center gap-1 text-left transition-colors hover:text-foreground"
        @click="store.toggleSort('name')"
      >
        名称
        <span v-if="store.sortKey === 'name'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
      <button
        class="flex items-center gap-1 transition-colors hover:text-foreground"
        @click="store.toggleSort('ext')"
      >
        类型
        <span v-if="store.sortKey === 'ext'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
      <button
        class="flex items-center gap-1 transition-colors hover:text-foreground"
        @click="store.toggleSort('size')"
      >
        大小
        <span v-if="store.sortKey === 'size'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
      <button
        class="flex items-center gap-1 transition-colors hover:text-foreground"
        @click="store.toggleSort('mtime')"
      >
        修改时间
        <span v-if="store.sortKey === 'mtime'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
    </div>

    <div ref="containerRef" class="flex-1 overflow-auto">
      <div
        v-if="store.loading && !store.items.length"
        class="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground"
      >
        <Loader2 class="size-5 animate-spin" />
        <span class="text-xs">加载中...</span>
      </div>
      <div v-else-if="store.error" class="px-3 py-6 text-sm text-red-500">
        {{ store.error }}
      </div>
      <div
        v-else-if="!store.filtered.length"
        class="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground"
      >
        <Search v-if="store.nameFilter" class="size-6 opacity-50" />
        <span class="text-sm">
          {{ store.showFavorites ? "暂无星标文件" : store.showRecent ? "暂无最近查看的文件" : store.nameFilter ? `没有匹配 "${store.nameFilter}" 的文件` : "暂无文件" }}
        </span>
        <span v-if="store.nameFilter" class="text-xs">试试清空过滤词或更换关键词</span>
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
              'grid h-full grid-cols-[1fr_80px_70px_120px] items-center gap-3 px-3 text-sm cursor-default transition-colors',
              isSelected(store.filtered[vi.index]!)
                ? 'bg-primary/8 border-l-2 border-primary'
                : isActive(store.filtered[vi.index]!)
                  ? 'bg-muted/60'
                  : 'hover:bg-muted/30',
              isSelected(store.filtered[vi.index]!)
                ? 'pl-[10px]'
                : 'pl-3',
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
                  isSelected(store.filtered[vi.index]!) ? 'text-primary' : 'text-muted-foreground',
                ]"
              />
              <span
                :class="[
                  'truncate',
                  isSelected(store.filtered[vi.index]!) ? 'font-medium' : '',
                ]"
                >{{ store.filtered[vi.index]!.name }}</span
              >
              <button
                class="shrink-0 rounded p-0.5 hover:bg-muted/60"
                :title="store.favoriteIds.has(store.filtered[vi.index]!.id) ? '取消星标' : '添加星标'"
                @click.stop="doToggleFavorite(store.filtered[vi.index]!.id)"
              >
                <Star
                  class="size-3"
                  :class="store.favoriteIds.has(store.filtered[vi.index]!.id)
                    ? 'fill-yellow-500 text-yellow-500'
                    : 'text-muted-foreground opacity-40 hover:opacity-80'"
                />
              </button>
            </div>
            <span class="text-xs text-muted-foreground">
              .{{ store.filtered[vi.index]!.ext || "—" }}
            </span>
            <span class="text-xs text-muted-foreground">
              {{ formatSize(store.filtered[vi.index]!.size) }}
            </span>
            <span class="text-xs text-muted-foreground">
              {{ formatDate(store.filtered[vi.index]!.mtime) }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="store.items.length"
      class="flex items-center gap-2 border-t border-border bg-muted/10 px-3 py-1.5 text-[10px] text-muted-foreground/70"
    >
      <template v-if="store.showFavorites">
        <span>{{ store.filtered.length }} 个星标文件</span>
      </template>
      <template v-else-if="store.showRecent">
        <span>{{ store.filtered.length }} 个最近查看</span>
      </template>
      <template v-else-if="store.nameFilter">
        <span>过滤 {{ store.filtered.length }}/{{ store.items.length }}</span>
      </template>
      <template v-else>
        <span>共 {{ store.items.length }} 个文件</span>
        <span class="text-muted-foreground/40">·</span>
        <span>双击打开</span>
        <span class="text-muted-foreground/40">·</span>
        <span>右键更多操作</span>
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
        打开
      </button>
      <button
        class="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-muted"
        @click="ctxAction('finder')"
      >
        <FolderOpen class="size-3.5" />
        在 Finder 中显示
      </button>
      <button
        class="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-muted"
        @click="ctxAction('copy')"
      >
        <Copy class="size-3.5" />
        复制路径
      </button>
    </ContextMenu>
  </div>
</template>
