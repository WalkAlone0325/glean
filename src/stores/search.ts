import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface SearchFilter {
  ext?: string | null;
  kind?: string | null;
  path_contains?: string | null;
  since?: number | null;
}

export interface SearchResult {
  id: number;
  path: string;
  name: string;
  ext: string | null;
  size: number;
  mtime: number;
  kind: string | null;
  snippet: string | null;
  rank: number;
}

export const useSearchStore = defineStore("search", () => {
  const query = ref("");
  const results = ref<SearchResult[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const filter = ref<SearchFilter>({});
  const selectedIndex = ref(0);
  const paletteOpen = ref(false);

  const hasResults = computed(() => results.value.length > 0);

  let timer: number | null = null;

  async function runSearch() {
    const q = query.value.trim();
    if (!q) {
      results.value = [];
      error.value = null;
      loading.value = false;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const out = await invoke<SearchResult[]>("search_files", {
        query: q,
        filter: filter.value,
        limit: 50,
      });
      results.value = out;
      selectedIndex.value = 0;
    } catch (e) {
      error.value = String(e);
      results.value = [];
    } finally {
      loading.value = false;
    }
  }

  function setQuery(v: string) {
    query.value = v;
    if (timer) window.clearTimeout(timer);
    timer = window.setTimeout(runSearch, 180);
  }

  function reset() {
    query.value = "";
    results.value = [];
    selectedIndex.value = 0;
    error.value = null;
  }

  function moveSelection(delta: number) {
    if (!results.value.length) return;
    const n = results.value.length;
    selectedIndex.value = (selectedIndex.value + delta + n) % n;
  }

  function openCurrent() {
    const item = results.value[selectedIndex.value];
    if (item) return invoke("open_file", { path: item.path });
    return Promise.resolve();
  }

  function openAt(index: number) {
    const item = results.value[index];
    if (item) return invoke("open_file", { path: item.path });
    return Promise.resolve();
  }

  return {
    query,
    results,
    loading,
    error,
    filter,
    selectedIndex,
    paletteOpen,
    hasResults,
    setQuery,
    reset,
    moveSelection,
    openCurrent,
    openAt,
    runSearch,
  };
});
