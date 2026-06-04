import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface FileEntry {
  id: number;
  path: string;
  name: string;
  ext: string | null;
  size: number;
  mtime: number;
  kind: string | null;
}

export interface RecentFile extends FileEntry {
  viewed_at: number;
}

export type SortKey = "mtime" | "name" | "size" | "ext";
export type SortDir = "asc" | "desc";

export const useFilesStore = defineStore("files", () => {
  const items = ref<FileEntry[]>([]);
  const recentItems = ref<RecentFile[]>([]);
  const showRecent = ref(false);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const sortKey = ref<SortKey>("mtime");
  const sortDir = ref<SortDir>("desc");
  const kindFilter = ref<string | null>(null);
  const selectedId = ref<number | null>(null);
  const nameFilter = ref("");

  const filtered = computed(() => {
    if (showRecent.value) return recentItems.value as unknown as FileEntry[];
    const q = nameFilter.value.trim().toLowerCase();
    if (!q) return items.value;
    return items.value.filter(
      (f) =>
        f.name.toLowerCase().includes(q) ||
        (f.ext && f.ext.toLowerCase().includes(q)) ||
        f.path.toLowerCase().includes(q),
    );
  });

  async function reload() {
    loading.value = true;
    error.value = null;
    try {
      const out = await invoke<FileEntry[]>("list_files", {
        sort: sortKey.value,
        dir: sortDir.value,
        kind: kindFilter.value,
        limit: 500,
        offset: 0,
      });
      items.value = out;
      if (selectedId.value && !out.some((f) => f.id === selectedId.value)) {
        selectedId.value = null;
      }
    } catch (e) {
      error.value = String(e);
      items.value = [];
    } finally {
      loading.value = false;
    }
  }

  function toggleSort(key: SortKey) {
    if (sortKey.value === key) {
      sortDir.value = sortDir.value === "desc" ? "asc" : "desc";
    } else {
      sortKey.value = key;
      sortDir.value = key === "name" || key === "ext" ? "asc" : "desc";
    }
    reload();
  }

  function setKindFilter(kind: string | null) {
    kindFilter.value = kind;
    reload();
  }

  function setNameFilter(v: string) {
    nameFilter.value = v;
  }

  function select(id: number | null) {
    selectedId.value = id;
    if (id !== null) {
      trackView(id);
    }
  }

  async function trackView(fileId: number) {
    try {
      await invoke("track_file_view", { fileId });
      if (showRecent.value) await loadRecentFiles();
    } catch (e) {
      // ignore
    }
  }

  async function loadRecentFiles() {
    try {
      recentItems.value = await invoke<RecentFile[]>("list_recent_files", { limit: 30 });
    } catch (e) {
      console.warn("load recent files failed:", e);
    }
  }

  function toggleRecent() {
    showRecent.value = !showRecent.value;
    if (showRecent.value) loadRecentFiles();
  }

  return {
    items,
    recentItems,
    showRecent,
    filtered,
    loading,
    error,
    sortKey,
    sortDir,
    kindFilter,
    nameFilter,
    selectedId,
    reload,
    toggleSort,
    setKindFilter,
    setNameFilter,
    select,
    loadRecentFiles,
    toggleRecent,
  };
});
