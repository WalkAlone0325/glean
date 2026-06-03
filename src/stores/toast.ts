import { defineStore } from "pinia";
import { ref } from "vue";

export interface Toast {
  id: number;
  message: string;
  kind: "success" | "error" | "info";
}

export const useToastStore = defineStore("toast", () => {
  const items = ref<Toast[]>([]);
  let counter = 0;

  function push(message: string, kind: Toast["kind"] = "info", durationMs = 1800) {
    const id = ++counter;
    items.value.push({ id, message, kind });
    window.setTimeout(() => remove(id), durationMs);
  }

  function remove(id: number) {
    items.value = items.value.filter((t) => t.id !== id);
  }

  return { items, push, remove };
});
