import { defineStore } from "pinia";
import { ref } from "vue";
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

export type SortKey = "mtime" | "name" | "size" | "ext";
export type SortDir = "asc" | "desc";

export const useFilesStore = defineStore("files", () => {
  const items = ref<FileEntry[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const sortKey = ref<SortKey>("mtime");
  const sortDir = ref<SortDir>("desc");
  const kindFilter = ref<string | null>(null);
  const selectedId = ref<number | null>(null);

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

  function select(id: number | null) {
    selectedId.value = id;
  }

  return {
    items,
    loading,
    error,
    sortKey,
    sortDir,
    kindFilter,
    selectedId,
    reload,
    toggleSort,
    setKindFilter,
    select,
  };
});
