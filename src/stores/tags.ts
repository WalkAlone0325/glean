import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface TagSummary {
  id: number;
  name: string;
  color: string | null;
  file_count: number;
}

export const useTagsStore = defineStore("tags", () => {
  const all = ref<TagSummary[]>([]);

  async function loadTags() {
    try {
      all.value = await invoke<TagSummary[]>("list_tags");
    } catch (e) {
      console.warn("load tags failed:", e);
    }
  }

  async function createTag(name: string, color?: string): Promise<TagSummary | null> {
    try {
      const tag = await invoke<TagSummary>("create_tag", { name, color: color || undefined });
      await loadTags();
      return tag;
    } catch (e) {
      console.warn("create tag failed:", e);
      return null;
    }
  }

  async function deleteTag(tagId: number) {
    try {
      await invoke("delete_tag", { tagId });
      all.value = all.value.filter((t) => t.id !== tagId);
    } catch (e) {
      console.warn("delete tag failed:", e);
    }
  }

  return { all, loadTags, createTag, deleteTag };
});
