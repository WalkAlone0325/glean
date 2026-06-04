<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useFilesStore } from "../stores/files";

const emit = defineEmits<{ close: [] }>();

const files = useFilesStore();
const step = ref<"welcome" | "folders" | "done">("welcome");
const folders = ref<string[]>(["~/Documents", "~/Downloads", "~/Desktop"]);
const indexing = ref(false);

async function onNext() {
  if (step.value === "welcome") {
    step.value = "folders";
  } else if (step.value === "folders") {
    indexing.value = true;
    await invoke("start_indexing");
    step.value = "done";
    files.reload();
  } else {
    emit("close");
  }
}

async function onAddFolder() {
  const dir = await openDialog({ directory: true, multiple: false, title: "选择索引文件夹" });
  if (dir && !folders.value.includes(dir)) {
    folders.value.push(dir);
  }
}

function onRemoveFolder(path: string) {
  folders.value = folders.value.filter((f) => f !== path);
}

function onSkip() {
  emit("close");
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-md">
    <div class="w-[480px] max-w-[90vw] rounded-2xl border border-border bg-background p-8 shadow-2xl">
      <!-- welcome -->
      <template v-if="step === 'welcome'">
        <div class="mb-6 text-center">
          <div class="mb-3 text-3xl">🌾</div>
          <h2 class="text-lg font-semibold">欢迎使用 Glean</h2>
          <p class="mt-2 text-sm leading-relaxed text-muted-foreground">
            本地的 AI 文件管家 — 智能检索、自动归档、语义搜索。<br />
            所有数据存储在本地，零服务器依赖。
          </p>
        </div>
        <div class="space-y-3 text-xs text-muted-foreground">
          <div class="flex items-start gap-3 rounded-lg bg-muted/40 p-3">
            <span class="text-lg leading-none">🔍</span>
            <div>
              <div class="font-medium text-foreground">语义检索</div>
              <div>用自然语言找文件：「上周那个融资 PDF 在哪？」</div>
            </div>
          </div>
          <div class="flex items-start gap-3 rounded-lg bg-muted/40 p-3">
            <span class="text-lg leading-none">🤖</span>
            <div>
              <div class="font-medium text-foreground">AI 助手</div>
              <div>对话式检索文件、自动打标签、一键重命名整理</div>
            </div>
          </div>
          <div class="flex items-start gap-3 rounded-lg bg-muted/40 p-3">
            <span class="text-lg leading-none">🔒</span>
            <div>
              <div class="font-medium text-foreground">隐私优先</div>
              <div>全部本地运行，你的文件不会离开你的电脑</div>
            </div>
          </div>
        </div>
      </template>

      <!-- folders -->
      <template v-if="step === 'folders'">
        <div class="mb-6 text-center">
          <h2 class="text-lg font-semibold">选择索引文件夹</h2>
          <p class="mt-1 text-sm text-muted-foreground">Glean 将扫描以下目录为文件建立索引</p>
        </div>
        <div class="mb-4 space-y-2">
          <div v-for="f in folders" :key="f" class="flex items-center gap-2 rounded-lg bg-muted/40 px-3 py-2 text-xs">
            <span class="flex-1 font-mono">{{ f }}</span>
            <button
              class="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-red-500"
              @click="onRemoveFolder(f)" aria-label="移除"
            >
              ✕
            </button>
          </div>
        </div>
        <button
          class="mb-4 w-full rounded-lg border border-dashed border-border px-3 py-2 text-xs text-muted-foreground hover:bg-muted/40"
          @click="onAddFolder"
        >
          + 添加文件夹
        </button>
      </template>

      <!-- done -->
      <template v-if="step === 'done'">
        <div class="mb-6 text-center">
          <div class="mb-3 text-3xl">🚀</div>
          <h2 class="text-lg font-semibold">准备就绪</h2>
          <p class="mt-2 text-sm text-muted-foreground">
            索引已在后台启动，你可以先浏览已索引的文件或开始对话
          </p>
        </div>
      </template>

      <div class="mt-8 flex items-center justify-between">
        <button class="text-xs text-muted-foreground hover:text-foreground" @click="onSkip">跳过</button>
        <button
          class="rounded-lg bg-primary px-6 py-2 text-sm font-medium text-primary-foreground hover:opacity-90 disabled:opacity-50"
          :disabled="indexing"
          @click="onNext"
        >
          {{ step === "done" ? "开始使用" : indexing ? "启动中..." : step === "welcome" ? "下一步" : "开始索引" }}
        </button>
      </div>
    </div>
  </div>
</template>
