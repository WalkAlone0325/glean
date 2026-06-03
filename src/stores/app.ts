import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface Stats {
  files: number;
  chunks: number;
  tags: number;
}

export const useAppStore = defineStore(
  "app",
  () => {
    const stats = ref<Stats>({ files: 0, chunks: 0, tags: 0 });
    const indexedFolders = ref<string[]>([]);
    const ready = ref(false);

    async function bootstrap() {
      try {
        stats.value = await invoke<Stats>("get_stats");
      } catch (e) {
        console.warn("failed to load stats:", e);
      } finally {
        ready.value = true;
      }
    }

    async function refreshStats() {
      stats.value = await invoke<Stats>("get_stats");
    }

    return { stats, indexedFolders, ready, bootstrap, refreshStats };
  },
  { persist: { pick: ["indexedFolders"] } },
);
