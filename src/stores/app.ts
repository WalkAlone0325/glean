import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface Stats {
  files: number;
  chunks: number;
  tags: number;
}

export interface EmbedProgress {
  embedded: number;
  total: number;
  current_chunk: number | null;
  phase: "Idle" | "Downloading" | "Embedding" | "Completed" | "Failed";
}

export const useAppStore = defineStore(
  "app",
  () => {
    const stats = ref<Stats>({ files: 0, chunks: 0, tags: 0 });
    const indexedFolders = ref<string[]>([]);
    const ready = ref(false);
    const theme = ref<"light" | "dark" | "system">("system");
    const locale = ref("zh-CN");
    const embedding = ref<EmbedProgress>({
      embedded: 0,
      total: 0,
      current_chunk: null,
      phase: "Idle",
    });

    async function bootstrap() {
      try {
        const [s, roots] = await Promise.all([
          invoke<Stats>("get_stats"),
          invoke<string[]>("get_indexed_roots").catch(() => [] as string[]),
        ]);
        stats.value = s;
        if (roots.length > 0) {
          indexedFolders.value = roots;
        }
      } catch (e) {
        console.warn("failed to load stats:", e);
      } finally {
        ready.value = true;
      }
    }

    async function refreshStats() {
      stats.value = await invoke<Stats>("get_stats");
      try {
        const roots = await invoke<string[]>("get_indexed_roots");
        if (roots.length > 0) indexedFolders.value = roots;
      } catch {
        /* ignore */
      }
    }

    function updateEmbedding(p: EmbedProgress) {
      embedding.value = p;
    }

    function applyTheme(t: "light" | "dark" | "system") {
      theme.value = t;
      const html = document.documentElement;
      if (t === "system") {
        html.classList.remove("light", "dark");
        const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        html.classList.add(prefersDark ? "dark" : "light");
      } else {
        html.classList.remove("light", "dark");
        html.classList.add(t);
      }
    }

    return { stats, indexedFolders, ready, embedding, theme, locale, applyTheme, bootstrap, refreshStats, updateEmbedding };
  },
  { persist: { pick: ["indexedFolders", "theme", "locale"] } },
);
