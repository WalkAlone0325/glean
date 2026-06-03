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

export type SearchMode = "auto" | "keyword" | "semantic";

export interface HybridResult extends SearchResult {
  source: "Both" | "VectorOnly" | "FtsOnly";
  vector_score: number;
  fts_rank: number;
}

function looksSemantic(q: string): boolean {
  const trimmed = q.trim();
  if (trimmed.length < 4) return false;
  const cjk = (trimmed.match(/[一-鿿]/g) || []).length;
  const letters = (trimmed.match(/[a-zA-Z]/g) || []).length;
  const spaces = (trimmed.match(/\s+/g) || []).length;
  if (cjk >= 4) return true;
  if (spaces >= 2 && letters >= 6) return true;
  return false;
}

export const useSearchStore = defineStore("search", () => {
  const query = ref("");
  const results = ref<HybridResult[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const filter = ref<SearchFilter>({});
  const selectedIndex = ref(0);
  const hoverIndex = ref<number | null>(null);
  const paletteOpen = ref(false);
  const mode = ref<SearchMode>("auto");

  const effectiveMode = computed(() => {
    if (mode.value !== "auto") return mode.value;
    return looksSemantic(query.value) ? "semantic" : "keyword";
  });

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
      const useSemantic = effectiveMode.value === "semantic";
      const cmd = useSemantic ? "hybrid_search_files" : "search_files";
      const out = (await invoke(cmd, {
        query: q,
        filter: filter.value,
        limit: useSemantic ? 30 : 50,
      })) as HybridResult[] | SearchResult[];
      const normalized: HybridResult[] = (out as HybridResult[]).map((r) => {
        const hybrid = r as HybridResult;
        if ("source" in hybrid && hybrid.source !== undefined) {
          return hybrid;
        }
        const fallback = r as SearchResult;
        return {
          ...fallback,
          source: "FtsOnly" as const,
          vector_score: 0,
          fts_rank: fallback.rank,
        };
      });
      results.value = normalized;
      selectedIndex.value = 0;
      hoverIndex.value = null;
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
    timer = window.setTimeout(runSearch, 220);
  }

  function reset() {
    query.value = "";
    results.value = [];
    selectedIndex.value = 0;
    hoverIndex.value = null;
    error.value = null;
  }

  function moveSelection(delta: number) {
    if (!results.value.length) return;
    const n = results.value.length;
    const next = selectedIndex.value + delta;
    if (next < 0) return;
    if (next >= n) return;
    selectedIndex.value = next;
    hoverIndex.value = null;
  }

  function setHover(idx: number | null) {
    hoverIndex.value = idx;
  }

  function activeIndex(): number {
    return hoverIndex.value ?? selectedIndex.value;
  }

  function openCurrent() {
    const idx = activeIndex();
    const item = results.value[idx];
    if (item) return invoke("open_file", { path: item.path });
    return Promise.resolve();
  }

  function openAt(index: number) {
    const item = results.value[index];
    if (item) return invoke("open_file", { path: item.path });
    return Promise.resolve();
  }

  function toggleMode() {
    const order: SearchMode[] = ["auto", "keyword", "semantic"];
    const i = order.indexOf(mode.value);
    mode.value = order[(i + 1) % order.length];
    if (query.value.trim()) runSearch();
  }

  return {
    query,
    results,
    loading,
    error,
    filter,
    selectedIndex,
    hoverIndex,
    paletteOpen,
    mode,
    effectiveMode,
    hasResults,
    setQuery,
    reset,
    moveSelection,
    setHover,
    activeIndex,
    openCurrent,
    openAt,
    runSearch,
    toggleMode,
  };
});
