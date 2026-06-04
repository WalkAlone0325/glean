<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { ExternalLink, Copy, Loader2, FolderOpen, Plus, X } from "@lucide/vue";
import { useFilesStore } from "../stores/files";
import { useToastStore } from "../stores/toast";
import { useTagsStore, type TagSummary } from "../stores/tags";
import { kindIcon, kindLabel, formatSize, formatDateTime } from "../utils/fileKind";
import { renderMarkdown } from "../utils/markdown";
import hljs from "highlight.js/lib/common";
import TagBadge from "./TagBadge.vue";

const { t } = useI18n();
const emit = defineEmits<{ close: [] }>();
const store = useFilesStore();
const toast = useToastStore();
const tags = useTagsStore();
const preview = ref<string | null>(null);
const previewLoading = ref(false);
const previewError = ref<string | null>(null);
const fileTags = ref<TagSummary[]>([]);
const showTagPicker = ref(false);
const newTagName = ref("");
const newTagColor = ref("");

tags.loadTags();

const file = computed(() => store.items.find((f) => f.id === store.selectedId) || null);
const Icon = computed(() => kindIcon(file.value?.kind));

const isImage = computed(() => file.value?.kind === "image");
const isPdf = computed(() => file.value?.kind === "pdf");
const isMarkdown = computed(() => file.value?.kind === "markdown");
const isCode = computed(() => file.value?.kind === "code");
const isHtml = computed(() => file.value?.kind === "html");
const isText = computed(() => {
  const k = file.value?.kind;
  return k === "text" || k === "markdown" || k === "code" || k === "data" || k === "html";
});

const imageUrl = computed(() => {
  if (!isImage.value || !file.value) return null;
  return convertFileSrc(file.value.path);
});

const pdfUrl = computed(() => {
  if (!isPdf.value || !file.value) return null;
  return convertFileSrc(file.value.path);
});

const htmlUrl = computed(() => {
  if (!isHtml.value || !file.value) return null;
  return convertFileSrc(file.value.path);
});

const previewLanguage = computed(() => {
  if (!file.value) return null;
  const ext = file.value.ext?.toLowerCase();
  if (!ext) return null;
  if (hljs.getLanguage(ext)) return ext;
  switch (ext) {
    case "ts":
    case "tsx":
      return "typescript";
    case "js":
    case "jsx":
      return "javascript";
    case "py":
      return "python";
    case "rb":
      return "ruby";
    case "rs":
      return "rust";
    case "go":
      return "go";
    case "kt":
      return "kotlin";
    case "sh":
    case "bash":
    case "zsh":
      return "bash";
    case "md":
    case "markdown":
      return "markdown";
    default:
      return null;
  }
});

const highlightedHtml = computed(() => {
  if (!preview.value) return "";
  if (isMarkdown.value) {
    return renderMarkdown(preview.value);
  }
  if (isCode.value && previewLanguage.value) {
    try {
      return `<pre class="rounded-md bg-zinc-900 p-3 text-[11px] overflow-x-auto"><code class="hljs language-${previewLanguage.value}">${
        hljs.highlight(preview.value, {
          language: previewLanguage.value,
          ignoreIllegals: true,
        }).value
      }</code></pre>`;
    } catch {
    }
  }
  if (isCode.value) {
    try {
      const auto = hljs.highlightAuto(preview.value);
      return `<pre class="rounded-md bg-zinc-900 p-3 text-[11px] overflow-x-auto"><code class="hljs">${auto.value}</code></pre>`;
    } catch {
    }
  }
  const escaped = preview.value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  return `<pre class="whitespace-pre-wrap break-all font-mono text-[11px] leading-snug">${escaped}</pre>`;
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
      maxBytes: 32 * 1024,
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
    toast.push(t('detail.copied'), "success");
  } catch {
    toast.push(t('toast.copy_failed'), "error");
  }
}

async function loadFileTags() {
  if (!file.value) {
    fileTags.value = [];
    return;
  }
  try {
    fileTags.value = await invoke<TagSummary[]>("get_file_tags", { fileId: file.value.id });
  } catch (e) {
    console.warn("load file tags failed:", e);
  }
}

async function addExistingTag(tagId: number) {
  if (!file.value || !file.value.id) return;
  const all = [...fileTags.value.map((t) => t.id), tagId];
  try {
    await invoke("set_file_tags", { fileId: file.value.id, tagIds: all });
    await loadFileTags();
  } catch (e) {
    toast.error(t('detail.add_tag_failed'));
  }
}

async function removeTag(tagId: number) {
  if (!file.value || !file.value.id) return;
  const all = fileTags.value.filter((t) => t.id !== tagId).map((t) => t.id);
  try {
    await invoke("set_file_tags", { fileId: file.value.id, tagIds: all });
    await loadFileTags();
  } catch (e) {
    toast.error(t('detail.remove_tag_failed'));
  }
}

async function addNewTag() {
  if (!file.value || !file.value.id || !newTagName.value.trim()) return;
  const tag = await tags.createTag(newTagName.value.trim(), newTagColor.value || undefined);
  if (tag) {
    const all = [...fileTags.value.map((t) => t.id), tag.id];
    try {
      await invoke("set_file_tags", { fileId: file.value.id, tagIds: all });
      await loadFileTags();
    } catch (e) {
      toast.error(t('detail.add_tag_failed'));
    }
  }
  newTagName.value = "";
  showTagPicker.value = false;
}

watch(
  () => file.value?.id,
  () => {
    loadPreview();
    loadFileTags();
    showTagPicker.value = false;
  },
  { immediate: true },
);
</script>

<template>
  <aside class="flex h-full w-72 flex-col border-l border-border">
    <div v-if="!file" class="flex flex-1 items-center justify-center text-xs text-muted-foreground">
      {{ t('filelist.no_preview') }}
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
          <button
            class="rounded p-0.5 text-muted-foreground hover:bg-muted transition-colors"
            :title="t('detail.close')"
            @click="emit('close')"
          >
            <X class="size-3.5" />
          </button>
        </div>
        <div class="mt-3 flex gap-1">
          <button
            class="flex flex-1 items-center justify-center gap-1 rounded-md bg-primary px-2 py-1.5 text-xs text-primary-foreground hover:opacity-90"
            @click="openExternally"
          >
            <ExternalLink class="size-3" />
            {{ t('filelist.open') }}
          </button>
          <button
            class="flex items-center justify-center rounded-md bg-muted px-2 py-1.5 text-xs hover:bg-muted/80"
            :title="t('filelist.reveal')"
            @click="revealInFinder"
          >
            <FolderOpen class="size-3" />
          </button>
          <button
            class="flex items-center justify-center rounded-md bg-muted px-2 py-1.5 text-xs hover:bg-muted/80"
            :title="t('filelist.copy_path')"
            @click="copyPath"
          >
            <Copy class="size-3" />
          </button>
        </div>
      </div>

      <div class="space-y-1.5 border-b border-border p-4 text-xs">
        <div class="flex justify-between gap-2">
          <span class="text-muted-foreground">{{ t('detail.size') }}</span>
          <span>{{ formatSize(file.size) }}</span>
        </div>
        <div class="flex justify-between gap-2">
          <span class="text-muted-foreground">{{ t('detail.mtime') }}</span>
          <span class="truncate">{{ formatDateTime(file.mtime) }}</span>
        </div>
        <div class="flex justify-between gap-2">
          <span class="text-muted-foreground">{{ t('detail.kind') }}</span>
          <span>{{ kindLabel(file.kind) }}</span>
        </div>
      </div>

      <div v-if="fileTags.length || showTagPicker" class="border-b border-border p-3">
        <div class="mb-1.5 flex items-center gap-2 text-[10px] text-muted-foreground">
          {{ t('detail.tags') }}
          <button
            v-if="!showTagPicker"
            class="ml-auto rounded p-0.5 hover:bg-muted"
            :title="t('detail.add_tag')"
            @click="showTagPicker = true"
          >
            <Plus class="size-3" />
          </button>
        </div>
        <div class="flex flex-wrap gap-1.5">
          <TagBadge
            v-for="t in fileTags"
            :key="t.id"
            :name="t.name"
            :color="t.color"
            :removable="true"
            :small="true"
            @remove="removeTag(t.id)"
          />
        </div>
        <div v-if="showTagPicker" class="mt-1.5 space-y-1.5">
          <div class="flex items-center gap-1.5">
            <select
              v-model="newTagColor"
              class="rounded border border-border bg-background px-1 py-0.5 text-[10px]"
            >
              <option value="">{{ t('detail.no_color') }}</option>
              <option value="red">{{ t('detail.color_red') }}</option>
              <option value="orange">{{ t('detail.color_orange') }}</option>
              <option value="yellow">{{ t('detail.color_yellow') }}</option>
              <option value="green">{{ t('detail.color_green') }}</option>
              <option value="blue">{{ t('detail.color_blue') }}</option>
              <option value="purple">{{ t('detail.color_purple') }}</option>
              <option value="pink">{{ t('detail.color_pink') }}</option>
            </select>
            <input
              v-model="newTagName"
              :placeholder="t('detail.new_tag_placeholder')"
              class="flex-1 rounded border border-border bg-background px-2 py-0.5 text-[11px] outline-none focus:border-primary"
              @keydown.enter="addNewTag"
            />
            <button
              class="rounded bg-primary px-2 py-0.5 text-[11px] text-primary-foreground hover:opacity-90"
              :disabled="!newTagName.trim()"
              @click="addNewTag"
            >
              {{ t('detail.add') }}
            </button>
            <button
              class="rounded px-1.5 py-0.5 text-[11px] hover:bg-muted"
              @click="showTagPicker = false"
            >
              {{ t('detail.cancel') }}
            </button>
          </div>
          <div class="flex flex-wrap gap-1">
            <button
              v-for="t in tags.all"
              v-show="!fileTags.find((ft) => ft.id === t.id)"
              :key="'all-' + t.id"
              class="rounded border border-border px-1.5 py-0.5 text-[10px] opacity-60 hover:opacity-100"
              @click="addExistingTag(t.id)"
            >
              {{ t.name }}
            </button>
          </div>
        </div>
      </div>

      <div class="flex-1 overflow-auto">
        <div v-if="isImage && imageUrl" class="flex h-full items-center justify-center bg-muted/30 p-4">
          <img :src="imageUrl" :alt="file.name" class="max-h-full max-w-full object-contain" />
        </div>
        <div v-else-if="isPdf && pdfUrl" class="h-full">
          <iframe :src="pdfUrl" class="h-full w-full border-0" :title="file.name" />
        </div>
        <div v-else-if="isHtml && htmlUrl" class="h-full">
          <iframe :src="htmlUrl" class="h-full w-full border-0" :title="file.name" />
        </div>
        <div v-else-if="isText" class="p-3">
          <div v-if="previewLoading" class="flex items-center gap-2 text-xs text-muted-foreground">
            <Loader2 class="size-3 animate-spin" />
            {{ t('filelist.preview_loading') }}
          </div>
          <div v-else-if="previewError" class="text-xs text-red-500">
            {{ previewError }}
          </div>
          <div
            v-else-if="preview"
            class="markdown-body"
            v-html="highlightedHtml"
          />
          <div v-else class="text-xs text-muted-foreground">{{ t('filelist.preview_error') }}</div>
        </div>
        <div
          v-else
          class="flex h-full items-center justify-center px-4 text-center text-xs text-muted-foreground"
        >
          {{ t('filelist.no_preview_type') }}
        </div>
      </div>
    </template>
  </aside>
</template>
