import { defineStore } from "pinia";
import { ref } from "vue";

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: number;
  message: string;
  kind: "success" | "error" | "info" | "warning";
  action?: ToastAction;
  durationMs: number;
}

export const useToastStore = defineStore("toast", () => {
  const items = ref<Toast[]>([]);
  let counter = 0;

  function push(
    message: string,
    kind: Toast["kind"] = "info",
    options: { durationMs?: number; action?: ToastAction } = {},
  ) {
    const id = ++counter;
    const duration = options.durationMs ?? (kind === "error" ? 4000 : 2000);
    const toast: Toast = {
      id,
      message,
      kind,
      action: options.action,
      durationMs: duration,
    };
    items.value.push(toast);
    window.setTimeout(() => remove(id), duration);
  }

  function success(message: string, action?: ToastAction) {
    push(message, "success", { action });
  }

  function error(message: string, action?: ToastAction) {
    push(message, "error", { action });
  }

  function info(message: string, action?: ToastAction) {
    push(message, "info", { action });
  }

  function remove(id: number) {
    items.value = items.value.filter((t) => t.id !== id);
  }

  function clear() {
    items.value = [];
  }

  return { items, push, success, error, info, remove, clear };
});
