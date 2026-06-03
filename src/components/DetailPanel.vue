<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { ExternalLink, Copy, Loader2, FolderOpen } from "@lucide/vue";
import { useFilesStore } from "../stores/files";
import { useToastStore } from "../stores/toast";
import { kindIcon, kindLabel, formatSize, formatDateTime } from "../utils/fileKind";

const store = useFilesStore();
const toast = useToastStore();
const preview = ref<string | null>(null);
const previewLoading = ref(false);
const previewError = ref<string | null>(null);

const file = computed(() => store.items.find((f) => f.id === store.selectedId) || null);
const Icon = computed(() => kindIcon(file.value?.kind));

const isImage = computed(() => file.value?.kind === "image");
const isText = computed(() => {
  const k = file.value?.kind;
  return k === "text" || k === "markdown" || k === "code" || k === "data" || k === "html";
});

const imageUrl = computed(() => {
  if (!isImage.value || !file.value) return null;
  return convertFileSrc(file.value.path);
});

async function loadPreview() {
  if (!file.value || !isText.value) {
    preview.value = null;
    return;
  }
  previewLoading.value = true;
  previewError.value = null;
  try {
    preview.value = await invoke<string>("read_text_preview", {
      path: file.value.path,
      maxBytes: 16 * 1024,
    });
  } catch (e) {
    previewError.value = String(e);
    preview.value = null;
  } finally {
    previewLoading.value = false;
  }
}

async function openExternally() {
  if (file.value) await invoke("open_file", { path: file.value.path });
}

async function revealInFinder() {
  if (file.value) await invoke("reveal_in_finder", { path: file.value.path });
}

async function copyPath() {
  if (!file.value) return;
  try {
    await navigator.clipboard.writeText(file.value.path);
    toast.push("已复制路径", "success");
  } catch {
    toast.push("复制失败", "error");
  }
}

watch(
  () => file.value?.id,
  () => loadPreview(),
  { immediate: true },
);
</script>

<template>
  <aside class="flex h-full w-72 flex-col border-l border-border">
    <div v-if="!file" class="flex flex-1 items-center justify-center text-xs text-muted-foreground">
      选择文件查看详情
    </div>
    <template v-else>
      <div class="border-b border-border p-4">
        <div class="flex items-start gap-2">
          <component :is="Icon" class="mt-0.5 size-5 shrink-0 text-muted-foreground" />
          <div class="min-w-0 flex-1">
            <div class="break-all text-sm font-medium">{{ file.name }}</div>
            <div class="mt-0.5 text-xs text-muted-foreground">
              .{{ file.ext || "?" }} · {{ kindLabel(file.kind) }}
            </div>
          </div>
        </div>
        <div class="mt-3 flex gap-1">
          <button
            @click="openExternally"
            class="flex flex-1 items-center justify-center gap-1 rounded-md bg-primary px-2 py-1.5 text-xs text-primary-foreground hover:opacity-90"
          >
            <ExternalLink class="size-3" />
            打开
          </button>
          <button
            @click="revealInFinder"
            class="flex items-center justify-center rounded-md bg-muted px-2 py-1.5 text-xs hover:bg-muted/80"
          >
            <FolderOpen class="size-3" />
          </button>
          <button
            @click="copyPath"
            class="flex items-center justify-center rounded-md bg-muted px-2 py-1.5 text-xs hover:bg-muted/80"
          >
            <Copy class="size-3" />
          </button>
        </div>
      </div>

      <div class="space-y-1.5 border-b border-border p-4 text-xs">
        <div class="flex justify-between gap-2">
          <span class="text-muted-foreground">大小</span>
          <span>{{ formatSize(file.size) }}</span>
        </div>
        <div class="flex justify-between gap-2">
          <span class="text-muted-foreground">修改</span>
          <span class="truncate">{{ formatDateTime(file.mtime) }}</span>
        </div>
        <div class="flex justify-between gap-2">
          <span class="text-muted-foreground">类型</span>
          <span>{{ file.kind || "—" }}</span>
        </div>
      </div>

      <div class="flex-1 overflow-auto">
        <div v-if="isImage && imageUrl" class="flex h-full items-center justify-center bg-muted/30 p-4">
          <img :src="imageUrl" :alt="file.name" class="max-h-full max-w-full object-contain" />
        </div>
        <div v-else-if="isText" class="p-3">
          <div v-if="previewLoading" class="flex items-center gap-2 text-xs text-muted-foreground">
            <Loader2 class="size-3 animate-spin" />
            加载预览...
          </div>
          <div v-else-if="previewError" class="text-xs text-red-500">
            {{ previewError }}
          </div>
          <pre
            v-else-if="preview"
            class="whitespace-pre-wrap break-all font-mono text-[11px] leading-snug text-foreground"
          >{{ preview }}</pre>
          <div v-else class="text-xs text-muted-foreground">无预览</div>
        </div>
        <div v-else class="flex h-full items-center justify-center px-4 text-center text-xs text-muted-foreground">
          该类型暂不支持预览，点击"打开"用默认应用查看
        </div>
      </div>
    </template>
  </aside>
</template>
