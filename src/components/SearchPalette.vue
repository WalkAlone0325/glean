<script setup lang="ts">
import { computed, nextTick, ref, useTemplateRef, watch } from "vue";
import { useSearchStore } from "../stores/search";
import { FileText, Search, Loader2 } from "@lucide/vue";
import { onClickOutside, useMagicKeys, whenever } from "@vueuse/core";

const store = useSearchStore();
const root = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
const itemRefs = useTemplateRef<HTMLLIElement[]>("itemRefs");

onClickOutside(root, () => (store.paletteOpen = false));

const keys = useMagicKeys();
whenever(keys["cmd+k"], () => {
  store.paletteOpen = !store.paletteOpen;
  if (store.paletteOpen) setTimeout(() => inputRef.value?.focus(), 30);
});
whenever(keys.escape, () => (store.paletteOpen = false));

watch(
  () => store.paletteOpen,
  (v) => {
    if (!v) store.reset();
  },
);

watch(
  () => store.selectedIndex,
  async (idx) => {
    await nextTick();
    const el = itemRefs.value?.[idx];
    if (el) el.scrollIntoView({ block: "nearest", behavior: "smooth" });
  },
);

function kindIcon(_kind: string | null) {
  return FileText;
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function highlight(snippet: string | null) {
  if (!snippet) return "";
  return snippet
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/\[([^\]]+)\]/g, '<mark class="rounded-sm bg-yellow-200/80 px-0.5 text-foreground dark:bg-yellow-500/40">$1</mark>');
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    store.moveSelection(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    store.moveSelection(-1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    store.openCurrent().then(() => (store.paletteOpen = false));
  }
}

const showPanel = computed(() => store.paletteOpen);
</script>

<template>
  <Teleport to="body">
    <div
      v-if="showPanel"
      class="fixed inset-0 z-50 flex items-start justify-center bg-black/30 backdrop-blur-sm"
    >
      <div
        ref="root"
        class="mt-24 flex max-h-[70vh] w-[640px] flex-col overflow-hidden rounded-xl border border-border bg-background shadow-2xl"
      >
        <div class="flex items-center gap-2 border-b border-border px-4 py-3">
          <Search class="size-5 text-muted-foreground" />
          <input
            ref="inputRef"
            :value="store.query"
            @input="store.setQuery(($event.target as HTMLInputElement).value)"
            @keydown="onKeydown"
            placeholder="搜索文件名、内容..."
            class="flex-1 bg-transparent text-base outline-none placeholder:text-muted-foreground"
          />
          <Loader2 v-if="store.loading" class="size-4 animate-spin text-muted-foreground" />
          <kbd class="text-xs text-muted-foreground">ESC</kbd>
        </div>

        <div class="flex-1 overflow-auto">
          <div v-if="store.error" class="px-4 py-6 text-sm text-red-500">
            {{ store.error }}
          </div>
          <div
            v-else-if="store.query && !store.hasResults && !store.loading"
            class="px-4 py-8 text-center text-sm text-muted-foreground"
          >
            没有匹配的文件
          </div>
          <ul v-else-if="store.hasResults" class="divide-y divide-border">
            <li
              v-for="(item, idx) in store.results"
              :key="item.id"
              ref="itemRefs"
              @click="store.openAt(idx).then(() => (store.paletteOpen = false))"
              @mouseenter="store.selectedIndex = idx"
              :class="[
                'flex cursor-pointer items-start gap-3 px-4 py-3',
                idx === store.selectedIndex ? 'bg-muted' : '',
              ]"
            >
              <component :is="kindIcon(item.kind)" class="mt-0.5 size-5 shrink-0 text-muted-foreground" />
              <div class="min-w-0 flex-1">
                <div class="truncate text-sm font-medium">{{ item.name }}</div>
                <div class="truncate text-xs text-muted-foreground">{{ item.path }}</div>
                <div
                  v-if="item.snippet"
                  class="mt-1 line-clamp-2 text-xs text-muted-foreground"
                  v-html="highlight(item.snippet)"
                />
              </div>
              <div class="shrink-0 text-right text-xs text-muted-foreground">
                <div>{{ formatSize(item.size) }}</div>
                <div>.{{ item.ext || "?" }}</div>
              </div>
            </li>
          </ul>
          <div v-else class="px-4 py-8 text-center text-sm text-muted-foreground">
            输入关键词开始搜索
          </div>
        </div>

        <div class="flex items-center justify-between border-t border-border px-4 py-2 text-xs text-muted-foreground">
          <span>{{ store.hasResults ? `${store.results.length} 个结果` : "FTS + jieba-rs" }}</span>
          <div class="flex items-center gap-3">
            <span><kbd class="font-mono">↑↓</kbd> 选择</span>
            <span><kbd class="font-mono">⏎</kbd> 打开</span>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
