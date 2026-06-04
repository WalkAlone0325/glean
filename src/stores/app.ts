import { defineStore } from "pinia";
import { computed, ref } from "vue";
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
        if (roots.length > 0 && indexedFolders.value.length === 0) {
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
    }

    function updateEmbedding(p: EmbedProgress) {
      embedding.value = p;
    }

    function applyTheme(t: "light" | "dark" | "system") {
      theme.value = t;
      const html = document.documentElement;
      html.classList.remove("light", "dark");
      if (t === "system") {
        const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
        html.classList.add(prefersDark ? "dark" : "light");
      } else {
        html.classList.add(t);
      }
    }

    if (typeof window !== "undefined") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      mq.addEventListener("change", (e) => {
        if (theme.value === "system") {
          const html = document.documentElement;
          html.classList.remove("light", "dark");
          html.classList.add(e.matches ? "dark" : "light");
        }
      });
    }

    const isFirstRun = computed(() => ready.value && indexedFolders.value.length === 0 && stats.value.files === 0);

    return { stats, indexedFolders, ready, embedding, theme, locale, isFirstRun, applyTheme, bootstrap, refreshStats, updateEmbedding };
  },
  { persist: { pick: ["indexedFolders", "theme", "locale"] } },
);
