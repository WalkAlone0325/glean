<script setup lang="ts">
import { computed, nextTick, ref, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useSearchStore } from "../stores/search";
import { FileText, Search, Loader2 } from "@lucide/vue";
import { onClickOutside, useMagicKeys, whenever } from "@vueuse/core";

const { t } = useI18n();
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
    if (!el) return;
    const container = el.closest(".search-scroll-container") as HTMLElement | null;
    if (!container) {
      el.scrollIntoView({ block: "nearest" });
      return;
    }
    const cRect = container.getBoundingClientRect();
    const eRect = el.getBoundingClientRect();
    const margin = 8;
    if (eRect.top < cRect.top + margin) {
      container.scrollTop -= cRect.top + margin - eRect.top;
    } else if (eRect.bottom > cRect.bottom - margin) {
      container.scrollTop += eRect.bottom - (cRect.bottom - margin);
    }
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
    .replace(/\[([^\]]+)\]/g, '<mark class="rounded-sm bg-accent/20 px-0.5 text-foreground">$1</mark>');
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
  } else if (e.key === "Tab") {
    e.preventDefault();
    store.toggleMode();
  }
}

const showPanel = computed(() => store.paletteOpen);

function sourceBadge(source: "Both" | "VectorOnly" | "FtsOnly") {
  if (source === "Both") return { text: t("search.source_both"), cls: "bg-blue-500/10 text-blue-600 dark:text-blue-400" };
  if (source === "VectorOnly") return { text: t("search.source_vector"), cls: "bg-purple-500/10 text-purple-600 dark:text-purple-400" };
  return { text: t("search.source_keyword"), cls: "bg-yellow-500/10 text-yellow-700 dark:text-yellow-400" };
}

const modeLabel = computed(() => {
  if (store.mode === "keyword") return t("search.mode_keyword");
  if (store.mode === "semantic") return t("search.mode_semantic");
  return t("search.mode_auto");
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="showPanel"
      class="fixed inset-0 z-50 flex items-start justify-center bg-black/20 backdrop-blur-sm"
    >
      <div
        ref="root"
        class="mt-[15vh] flex max-h-[60vh] w-[620px] flex-col overflow-hidden rounded-xl border border-border/60 bg-background/95 backdrop-blur-xl shadow-2xl animate-[fade-in_0.12s_ease-out]"
      >
        <div class="flex items-center gap-3 border-b border-border/50 px-4 py-3">
          <Search class="size-4 text-muted-foreground/50" />
          <input
            ref="inputRef"
            :value="store.query"
            :placeholder="t('search.placeholder_expanded')"
            class="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground/40"
            @input="store.setQuery(($event.target as HTMLInputElement).value)"
            @keydown="onKeydown"
          />
          <Loader2 v-if="store.loading" class="size-3.5 animate-spin text-muted-foreground/50" />
          <kbd class="text-[10px] text-muted-foreground/50 font-mono border border-border/40 rounded px-1.5 py-0.5">ESC</kbd>
        </div>

        <div class="search-scroll-container flex-1 overflow-auto">
          <div v-if="store.error" class="px-4 py-6 text-sm text-destructive">
            {{ store.error }}
          </div>
          <div
            v-else-if="store.loading && store.query"
            class="flex items-center justify-center gap-2 px-4 py-8 text-sm text-muted-foreground/60"
          >
            <Loader2 class="size-4 animate-spin" />
            {{ t('search.searching') }}
          </div>
          <div
            v-else-if="store.query && !store.hasResults"
            class="px-4 py-8 text-center text-sm text-muted-foreground/60"
          >
            {{ t('search.no_results') }}
          </div>
          <ul v-else-if="store.hasResults" class="py-1">
            <li
              v-for="(item, idx) in store.results"
              :key="item.id"
              ref="itemRefs"
              :class="[
                'flex cursor-pointer items-start gap-3 px-4 py-2.5 mx-1 rounded-lg transition-colors',
                idx === store.hoverIndex || (store.hoverIndex === null && idx === store.selectedIndex)
                  ? 'bg-accent/10'
                  : '',
              ]"
              @click="store.openAt(idx).then(() => (store.paletteOpen = false))"
              @mouseenter="store.setHover(idx)"
              @mouseleave="store.setHover(null)"
            >
              <component :is="kindIcon(item.kind)" class="mt-0.5 size-5 shrink-0 text-muted-foreground/50" />
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="truncate text-sm font-medium">{{ item.name }}</span>
                  <span
                    v-if="'source' in item"
                    :class="['rounded px-1.5 py-0.5 text-[10px] font-medium', sourceBadge(item.source).cls]"
                  >
                    {{ sourceBadge(item.source).text }}
                  </span>
                </div>
                <div class="truncate text-xs text-muted-foreground/60">{{ item.path }}</div>
                <div
                  v-if="item.snippet"
                  class="mt-1 line-clamp-2 text-xs text-muted-foreground/70"
                  v-html="highlight(item.snippet)"
                />
              </div>
              <div class="shrink-0 text-right text-xs text-muted-foreground/50">
                <div>{{ formatSize(item.size) }}</div>
                <div class="text-[10px]">.{{ item.ext || "?" }}</div>
              </div>
            </li>
          </ul>
          <div v-else class="px-4 py-8 text-center text-sm text-muted-foreground/50">
            {{ t('search.start_hint') }}
          </div>
        </div>

        <div class="flex items-center justify-between border-t border-border/50 px-4 py-2 text-xs text-muted-foreground/60">
          <div class="flex items-center gap-3">
            <button
              class="flex items-center gap-1 rounded px-1.5 py-0.5 hover:bg-muted/60 transition-colors"
              :title="t('search.current_mode', { mode: modeLabel })"
              @click="store.toggleMode()"
            >
              <span class="font-medium text-foreground/80">{{ modeLabel }}</span>
              <span v-if="store.mode === 'auto'" class="text-[10px] text-muted-foreground/50">→ {{ store.effectiveMode === "semantic" ? t("search.mode_semantic") : t("search.mode_keyword") }}</span>
            </button>
            <span v-if="store.hasResults" class="tabular-nums">{{ t('search.results_count', { n: store.results.length }) }}</span>
          </div>
          <div class="flex items-center gap-3">
            <span><kbd class="font-mono text-[10px]">Tab</kbd> {{ t('search.hint_mode') }}</span>
            <span><kbd class="font-mono text-[10px]">↑↓</kbd> {{ t('search.hint_nav') }}</span>
            <span><kbd class="font-mono text-[10px]">⏎</kbd> {{ t('search.hint_open') }}</span>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
