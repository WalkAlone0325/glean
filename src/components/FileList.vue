<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { invoke } from "@tauri-apps/api/core";
import { useFilesStore, type FileEntry } from "../stores/files";
import { Loader2 } from "@lucide/vue";
import { kindIcon, formatSize, formatDate } from "../utils/fileKind";

const store = useFilesStore();
const containerRef = useTemplateRef<HTMLDivElement>("containerRef");
const hoverId = ref<number | null>(null);

const rowHeight = 44;

const virtualizer = useVirtualizer(
  computed(() => ({
    count: store.items.length,
    getScrollElement: () => containerRef.value,
    estimateSize: () => rowHeight,
    overscan: 16,
  })),
);

const items = computed(() => virtualizer.value.getVirtualItems());

function isActive(item: FileEntry) {
  return item.id === store.selectedId || item.id === hoverId.value;
}

function onDoubleClick(item: FileEntry) {
  invoke("open_file", { path: item.path });
}

watch(() => store.items, () => virtualizer.value.scrollToIndex(0), { flush: "post" });
</script>

<template>
  <div class="flex h-full flex-col">
    <div
      class="grid grid-cols-[1fr_80px_70px_120px] gap-3 border-b border-border px-3 py-2 text-xs font-medium text-muted-foreground"
    >
      <button
        @click="store.toggleSort('name')"
        class="flex items-center gap-1 text-left hover:text-foreground"
      >
        名称
        <span v-if="store.sortKey === 'name'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
      <button
        @click="store.toggleSort('ext')"
        class="flex items-center gap-1 hover:text-foreground"
      >
        类型
        <span v-if="store.sortKey === 'ext'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
      <button
        @click="store.toggleSort('size')"
        class="flex items-center gap-1 hover:text-foreground"
      >
        大小
        <span v-if="store.sortKey === 'size'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
      <button
        @click="store.toggleSort('mtime')"
        class="flex items-center gap-1 hover:text-foreground"
      >
        修改时间
        <span v-if="store.sortKey === 'mtime'" class="text-foreground">
          {{ store.sortDir === "asc" ? "↑" : "↓" }}
        </span>
      </button>
    </div>

    <div ref="containerRef" class="flex-1 overflow-auto">
      <div v-if="store.loading" class="flex items-center justify-center py-10 text-muted-foreground">
        <Loader2 class="mr-2 size-4 animate-spin" />
        加载中...
      </div>
      <div v-else-if="store.error" class="px-3 py-6 text-sm text-red-500">
        {{ store.error }}
      </div>
      <div
        v-else-if="!store.items.length"
        class="flex h-full items-center justify-center text-sm text-muted-foreground"
      >
        暂无文件
      </div>
      <div
        v-else
        :style="{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }"
      >
        <div
          v-for="vi in items"
          :key="store.items[vi.index]!.id"
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
            @click="store.select(store.items[vi.index]!.id)"
            @dblclick="onDoubleClick(store.items[vi.index]!)"
            @mouseenter="hoverId = store.items[vi.index]!.id"
            @mouseleave="hoverId = null"
            :class="[
              'grid h-full grid-cols-[1fr_80px_70px_120px] items-center gap-3 px-3 text-sm',
              isActive(store.items[vi.index]!) ? 'bg-muted' : '',
            ]"
          >
            <div class="flex min-w-0 items-center gap-2">
              <component
                :is="kindIcon(store.items[vi.index]!.kind)"
                class="size-4 shrink-0 text-muted-foreground"
              />
              <span class="truncate">{{ store.items[vi.index]!.name }}</span>
            </div>
            <span class="text-xs text-muted-foreground">
              .{{ store.items[vi.index]!.ext || "—" }}
            </span>
            <span class="text-xs text-muted-foreground">
              {{ formatSize(store.items[vi.index]!.size) }}
            </span>
            <span class="text-xs text-muted-foreground">
              {{ formatDate(store.items[vi.index]!.mtime) }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
