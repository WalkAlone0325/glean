<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { useAppStore } from "../stores/app";
import { useFilesStore } from "../stores/files";

const { t } = useI18n();
const emit = defineEmits<{ close: [] }>();

const app = useAppStore();
const files = useFilesStore();
const step = ref<"welcome" | "folders" | "done">("welcome");
const folders = ref<string[]>([]);

onMounted(async () => {
  const h = await homeDir();
  folders.value = [
    `${h}/Desktop`,
    `${h}/Downloads`,
    `${h}/Documents`,
  ];
});
const indexing = ref(false);

async function onNext() {
  if (step.value === "welcome") {
    step.value = "folders";
  } else if (step.value === "folders") {
    indexing.value = true;
    const paths = folders.value.filter(Boolean);
    if (paths.length) {
      await invoke("start_indexing", { paths });
      app.indexedFolders = Array.from(new Set([...(app.indexedFolders || []), ...paths]));
    }
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
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-md">
    <div class="w-[460px] max-w-[90vw] rounded-xl border border-border/60 bg-background/95 backdrop-blur-xl p-7 shadow-2xl animate-[fade-in_0.2s_ease-out]">
      <template v-if="step === 'welcome'">
        <div class="mb-6 text-center">
          <div class="mb-4 mx-auto flex size-14 items-center justify-center rounded-full bg-accent/10">
            <span class="text-2xl">🌾</span>
          </div>
          <h2 class="text-lg font-semibold">{{ t('onboarding.welcome_title') }}</h2>
          <p class="mt-2 text-sm leading-relaxed text-muted-foreground/80">
            {{ t('onboarding.welcome_desc') }}
          </p>
        </div>
        <div class="space-y-2.5 text-xs text-muted-foreground">
          <div class="flex items-start gap-3 rounded-lg bg-muted/30 p-3">
            <span class="mt-0.5 text-lg leading-none">🔍</span>
            <div>
              <div class="font-medium text-foreground/90">{{ t('onboarding.feature_search') }}</div>
              <div class="mt-0.5 text-muted-foreground/70">{{ t('onboarding.feature_search_desc') }}</div>
            </div>
          </div>
          <div class="flex items-start gap-3 rounded-lg bg-muted/30 p-3">
            <span class="mt-0.5 text-lg leading-none">🤖</span>
            <div>
              <div class="font-medium text-foreground/90">{{ t('onboarding.feature_agent') }}</div>
              <div class="mt-0.5 text-muted-foreground/70">{{ t('onboarding.feature_agent_desc') }}</div>
            </div>
          </div>
          <div class="flex items-start gap-3 rounded-lg bg-muted/30 p-3">
            <span class="mt-0.5 text-lg leading-none">🔒</span>
            <div>
              <div class="font-medium text-foreground/90">{{ t('onboarding.feature_privacy') }}</div>
              <div class="mt-0.5 text-muted-foreground/70">{{ t('onboarding.feature_privacy_desc') }}</div>
            </div>
          </div>
        </div>
      </template>

      <template v-if="step === 'folders'">
        <div class="mb-6 text-center">
          <h2 class="text-lg font-semibold">{{ t('onboarding.folder_title') }}</h2>
          <p class="mt-1 text-sm text-muted-foreground/70">{{ t('onboarding.folder_desc') }}</p>
        </div>
        <div class="mb-3 space-y-1.5">
          <div v-for="f in folders" :key="f" class="flex items-center gap-2 rounded-md bg-muted/30 px-3 py-2 text-xs">
            <span class="flex-1 font-mono truncate">{{ f }}</span>
            <button
              class="rounded p-0.5 text-muted-foreground/60 hover:bg-muted hover:text-destructive transition-colors"
              :aria-label="t('onboarding.folder_remove')" @click="onRemoveFolder(f)"
            >
              ✕
            </button>
          </div>
        </div>
        <button
          class="mb-4 w-full rounded-md border border-dashed border-border/60 px-3 py-2 text-xs text-muted-foreground/70 hover:bg-muted/30 transition-colors"
          @click="onAddFolder"
        >
          {{ t('onboarding.folder_add') }}
        </button>
      </template>

      <template v-if="step === 'done'">
        <div class="mb-6 text-center">
          <div class="mb-4 mx-auto flex size-14 items-center justify-center rounded-full bg-emerald-500/10">
            <span class="text-2xl">🚀</span>
          </div>
          <h2 class="text-lg font-semibold">{{ t('onboarding.done_title') }}</h2>
          <p class="mt-2 text-sm text-muted-foreground/80">
            {{ t('onboarding.done_desc') }}
          </p>
        </div>
      </template>

      <div class="mt-6 flex items-center justify-between">
        <button class="text-xs text-muted-foreground/60 hover:text-foreground transition-colors" @click="onSkip">{{ t('onboarding.skip') }}</button>
        <button
          class="rounded-lg bg-accent px-5 py-2 text-sm font-medium text-accent-foreground shadow-xs hover:brightness-110 active:scale-[0.98] transition-all disabled:opacity-50"
          :disabled="indexing"
          @click="onNext"
        >
          {{ step === "done" ? t('onboarding.start_using') : indexing ? t('onboarding.starting') : step === "welcome" ? t('onboarding.next') : t('onboarding.start') }}
        </button>
      </div>
    </div>
  </div>
</template>
