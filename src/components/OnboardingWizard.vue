<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { useAppStore } from "../stores/app";
import { ArrowRight, FolderOpen, Loader2, Plus, X } from "@lucide/vue";

const { t } = useI18n();
const emit = defineEmits<{ close: []; indexingStarted: [] }>();

const app = useAppStore();
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
    emit("indexingStarted");
    const paths = folders.value.filter(Boolean);
    if (paths.length) {
      await invoke("start_indexing", { paths });
      app.indexedFolders = Array.from(new Set([...(app.indexedFolders || []), ...paths]));
    }
    step.value = "done";
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
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="relative w-[480px] max-w-[90vw] overflow-hidden rounded-2xl border border-border/50 bg-background shadow-2xl animate-[fade-in_0.2s_ease-out]">
      <div class="p-8">
        <template v-if="step === 'welcome'">
          <div class="mb-7 text-center">
            <div class="mb-5 mx-auto flex size-16 items-center justify-center rounded-2xl bg-gradient-to-br from-accent/20 to-accent/5 ring-1 ring-accent/10">
              <span class="text-3xl">🌾</span>
            </div>
            <h2 class="text-xl font-bold tracking-tight">{{ t('onboarding.welcome_title') }}</h2>
            <p class="mt-2 text-sm leading-relaxed text-muted-foreground">
              {{ t('onboarding.welcome_desc') }}
            </p>
          </div>
          <div class="space-y-2.5">
            <div class="flex items-start gap-3.5 rounded-xl bg-muted/40 px-4 py-3.5 ring-1 ring-border/30">
              <span class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-blue-500/10 text-base leading-none">🔍</span>
              <div>
                <div class="text-sm font-semibold text-foreground/90">{{ t('onboarding.feature_search') }}</div>
                <div class="mt-0.5 text-xs leading-relaxed text-muted-foreground/70">{{ t('onboarding.feature_search_desc') }}</div>
              </div>
            </div>
            <div class="flex items-start gap-3.5 rounded-xl bg-muted/40 px-4 py-3.5 ring-1 ring-border/30">
              <span class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-purple-500/10 text-base leading-none">🤖</span>
              <div>
                <div class="text-sm font-semibold text-foreground/90">{{ t('onboarding.feature_agent') }}</div>
                <div class="mt-0.5 text-xs leading-relaxed text-muted-foreground/70">{{ t('onboarding.feature_agent_desc') }}</div>
              </div>
            </div>
            <div class="flex items-start gap-3.5 rounded-xl bg-muted/40 px-4 py-3.5 ring-1 ring-border/30">
              <span class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-emerald-500/10 text-base leading-none">🔒</span>
              <div>
                <div class="text-sm font-semibold text-foreground/90">{{ t('onboarding.feature_privacy') }}</div>
                <div class="mt-0.5 text-xs leading-relaxed text-muted-foreground/70">{{ t('onboarding.feature_privacy_desc') }}</div>
              </div>
            </div>
          </div>
        </template>

        <template v-if="step === 'folders'">
          <div class="mb-6 text-center">
            <div class="mb-4 mx-auto flex size-12 items-center justify-center rounded-xl bg-accent/10">
              <span class="text-xl">📁</span>
            </div>
            <h2 class="text-lg font-bold tracking-tight">{{ t('onboarding.folder_title') }}</h2>
            <p class="mt-1 text-sm text-muted-foreground">{{ t('onboarding.folder_desc') }}</p>
          </div>
          <div class="mb-3 max-h-[200px] space-y-1 overflow-auto">
            <div v-for="f in folders" :key="f" class="group flex items-center gap-2 rounded-lg bg-muted/30 px-3 py-2.5 text-xs ring-1 ring-border/20">
              <FolderOpen class="size-3.5 shrink-0 text-muted-foreground/50" />
              <span class="min-w-0 flex-1 truncate font-mono text-muted-foreground/80">{{ f }}</span>
              <button
                class="shrink-0 rounded-md p-1 text-muted-foreground/40 opacity-0 group-hover:opacity-100 hover:bg-muted hover:text-destructive transition-all"
                :aria-label="t('onboarding.folder_remove')" @click="onRemoveFolder(f)"
              >
                <X class="size-3" />
              </button>
            </div>
          </div>
          <button
            class="flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-border/50 px-3 py-2.5 text-xs text-muted-foreground/60 hover:border-border hover:bg-muted/20 hover:text-muted-foreground transition-colors"
            @click="onAddFolder"
          >
            <Plus class="size-3.5" />
            {{ t('onboarding.folder_add') }}
          </button>
        </template>

        <template v-if="step === 'done'">
          <div class="mb-6 text-center">
            <div class="mb-5 mx-auto flex size-16 items-center justify-center rounded-2xl bg-gradient-to-br from-emerald-500/20 to-emerald-500/5 ring-1 ring-emerald-500/10">
              <span class="text-3xl">🚀</span>
            </div>
            <h2 class="text-xl font-bold tracking-tight">{{ t('onboarding.done_title') }}</h2>
            <p class="mt-2 text-sm leading-relaxed text-muted-foreground">
              {{ t('onboarding.done_desc') }}
            </p>
          </div>
        </template>

        <div v-if="step !== 'done'" class="mt-7 flex items-center justify-between">
          <button class="text-xs text-muted-foreground/50 hover:text-foreground transition-colors" @click="onSkip">{{ t('onboarding.skip') }}</button>
          <button
            class="inline-flex items-center gap-1.5 rounded-xl bg-accent px-5 py-2.5 text-sm font-semibold text-accent-foreground shadow-sm hover:brightness-110 active:scale-[0.97] transition-all disabled:opacity-40"
            :disabled="indexing"
            @click="onNext"
          >
            <span>{{ step === "folders" && indexing ? '' : '' }}</span>
            {{ indexing ? t('onboarding.starting') : step === "welcome" ? t('onboarding.next') : t('onboarding.start') }}
            <ArrowRight v-if="!indexing" class="size-3.5" />
            <Loader2 v-else class="size-3.5 animate-spin" />
          </button>
        </div>
        <div v-else class="mt-7 flex justify-center">
          <button
            class="inline-flex items-center gap-1.5 rounded-xl bg-accent px-6 py-2.5 text-sm font-semibold text-accent-foreground shadow-sm hover:brightness-110 active:scale-[0.97] transition-all"
            @click="onNext"
          >
            {{ t('onboarding.start_using') }}
            <ArrowRight class="size-3.5" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
