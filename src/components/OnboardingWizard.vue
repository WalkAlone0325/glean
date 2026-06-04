<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useFilesStore } from "../stores/files";

const { t } = useI18n();
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
  const dir = await openDialog({ directory: true, multiple: false, title: t("onboarding.folder_title") });
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
          <h2 class="text-lg font-semibold">{{ t('onboarding.welcome_title') }}</h2>
          <p class="mt-2 text-sm leading-relaxed text-muted-foreground">
            {{ t('onboarding.welcome_desc') }}
          </p>
        </div>
        <div class="space-y-3 text-xs text-muted-foreground">
          <div class="flex items-start gap-3 rounded-lg bg-muted/40 p-3">
            <span class="text-lg leading-none">🔍</span>
            <div>
              <div class="font-medium text-foreground">{{ t('onboarding.feature_search') }}</div>
              <div>{{ t('onboarding.feature_search_desc') }}</div>
            </div>
          </div>
          <div class="flex items-start gap-3 rounded-lg bg-muted/40 p-3">
            <span class="text-lg leading-none">🤖</span>
            <div>
              <div class="font-medium text-foreground">{{ t('onboarding.feature_agent') }}</div>
              <div>{{ t('onboarding.feature_agent_desc') }}</div>
            </div>
          </div>
          <div class="flex items-start gap-3 rounded-lg bg-muted/40 p-3">
            <span class="text-lg leading-none">🔒</span>
            <div>
              <div class="font-medium text-foreground">{{ t('onboarding.feature_privacy') }}</div>
              <div>{{ t('onboarding.feature_privacy_desc') }}</div>
            </div>
          </div>
        </div>
      </template>

      <!-- folders -->
      <template v-if="step === 'folders'">
        <div class="mb-6 text-center">
          <h2 class="text-lg font-semibold">{{ t('onboarding.folder_title') }}</h2>
          <p class="mt-1 text-sm text-muted-foreground">{{ t('onboarding.folder_desc') }}</p>
        </div>
        <div class="mb-4 space-y-2">
          <div v-for="f in folders" :key="f" class="flex items-center gap-2 rounded-lg bg-muted/40 px-3 py-2 text-xs">
            <span class="flex-1 font-mono">{{ f }}</span>
            <button
              class="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-red-500"
              :aria-label="t('onboarding.folder_remove')" @click="onRemoveFolder(f)"
            >
              ✕
            </button>
          </div>
        </div>
        <button
          class="mb-4 w-full rounded-lg border border-dashed border-border px-3 py-2 text-xs text-muted-foreground hover:bg-muted/40"
          @click="onAddFolder"
        >
          {{ t('onboarding.folder_add') }}
        </button>
      </template>

      <!-- done -->
      <template v-if="step === 'done'">
        <div class="mb-6 text-center">
          <div class="mb-3 text-3xl">🚀</div>
          <h2 class="text-lg font-semibold">{{ t('onboarding.done_title') }}</h2>
          <p class="mt-2 text-sm text-muted-foreground">
            {{ t('onboarding.done_desc') }}
          </p>
        </div>
      </template>

      <div class="mt-8 flex items-center justify-between">
        <button class="text-xs text-muted-foreground hover:text-foreground" @click="onSkip">{{ t('onboarding.skip') }}</button>
        <button
          class="rounded-lg bg-primary px-6 py-2 text-sm font-medium text-primary-foreground hover:opacity-90 disabled:opacity-50"
          :disabled="indexing"
          @click="onNext"
        >
          {{ step === "done" ? t('onboarding.start_using') : indexing ? t('onboarding.starting') : step === "welcome" ? t('onboarding.next') : t('onboarding.start') }}
        </button>
      </div>
    </div>
  </div>
</template>
