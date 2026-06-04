<script setup lang="ts">
import { onMounted, ref, reactive } from "vue";
import { useI18n } from "vue-i18n";
import {
  X, Eye, EyeOff, Loader2, CheckCircle2, AlertCircle,
  HardDrive, FolderTree, Palette, Languages, Keyboard, Lock,
  FileText, Hash, Tags, Sun, Moon, Monitor,
} from "@lucide/vue";
import { onClickOutside, useMagicKeys, whenever } from "@vueuse/core";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore, providerPresets } from "../stores/settings";
import { useAppStore } from "../stores/app";
import { useToastStore } from "../stores/toast";

const { t } = useI18n();
const emit = defineEmits<{ close: [] }>();

const settings = useSettingsStore();
const app = useAppStore();
const toast = useToastStore();
const root = ref<HTMLElement | null>(null);
const showKey = ref(false);
const testing = ref(false);
const testResult = ref<{ ok: boolean; message: string } | null>(null);
const saving = ref(false);
const currentTab = ref("provider");

const stats = reactive({ files: 0, chunks: 0, tags: 0 });
const indexedRoots = ref<string[]>([]);
const indexing = ref(false);
const paused = ref(false);

const tabs = [
  { id: "provider", icon: HardDrive, label: "settings.llm_provider" },
  { id: "indexing", icon: FolderTree, label: "settings.indexing" },
  { id: "appearance", icon: Palette, label: "settings.appearance" },
  { id: "shortcuts", icon: Keyboard, label: "settings.shortcuts" },
  { id: "ignore", icon: FileText, label: "settings.ignore_rules" },
  { id: "privacy", icon: Lock, label: "settings.privacy" },
  { id: "stats", icon: Hash, label: "settings.stats" },
];

onClickOutside(root, () => emit("close"));
const keys = useMagicKeys();
whenever(keys.escape, () => emit("close"));

onMounted(async () => {
  settings.load();
  await loadStats();
  await loadIndexedRoots();
  await loadIndexStatus();
  await loadIgnoreRules();
});

async function loadStats() {
  try {
    const s = await invoke<{ files: number; chunks: number; tags: number }>("get_stats");
    stats.files = s.files;
    stats.chunks = s.chunks;
    stats.tags = s.tags;
  } catch { /* ignore */ }
}

async function loadIndexedRoots() {
  try {
    indexedRoots.value = await invoke<string[]>("get_indexed_roots");
  } catch { /* ignore */ }
}

async function loadIndexStatus() {
  try {
    indexing.value = await invoke<boolean>("is_indexing");
    paused.value = await invoke<boolean>("is_paused");
  } catch { /* ignore */ }
}

async function onStartIndex() {
  try {
    await invoke("start_indexing");
    indexing.value = true;
    paused.value = false;
    toast.success(t('settings.index_started'));
  } catch (e) {
    toast.error(t('settings.index_failed', { msg: String(e) }));
  }
}

async function onCancelIndex() {
  try {
    await invoke("cancel_indexing");
    indexing.value = false;
    paused.value = false;
  } catch (e) {
    toast.error(t('settings.index_cancel_failed', { msg: String(e) }));
  }
}

async function onPauseIndex() {
  try {
    await invoke("pause_indexing");
    paused.value = true;
  } catch { /* ignore */ }
}

async function onResumeIndex() {
  try {
    await invoke("resume_indexing");
    paused.value = false;
  } catch { /* ignore */ }
}

const ignoreRules = ref("");

async function loadIgnoreRules() {
  try {
    const rules = await invoke<string[]>("get_ignore_rules");
    ignoreRules.value = rules.join("\n");
  } catch { /* ignore */ }
}

async function onSaveIgnoreRules() {
  try {
    const lines = ignoreRules.value.split("\n").map((l) => l.trim()).filter(Boolean);
    await invoke("set_ignore_rules", { rules: lines });
    toast.success(t('settings.ignore_saved'));
  } catch (e) {
    toast.error(t('settings.ignore_failed', { msg: String(e) }));
  }
}

async function onSave() {
  saving.value = true;
  try {
    await settings.save();
    toast.success(t('settings.saved'));
    emit("close");
  } catch (e) {
    toast.error(t('settings.save_failed', { msg: String(e) }));
  } finally {
    saving.value = false;
  }
}

async function onTest() {
  if (!settings.config.api_key) {
    testResult.value = { ok: false, message: t('settings.test_empty_key') };
    return;
  }
  testing.value = true;
  testResult.value = null;
  try {
    await settings.save();
    const reply = await settings.testConnection();
    testResult.value = { ok: true, message: t('settings.test_ok', { reply }) };
  } catch (e) {
    testResult.value = { ok: false, message: String(e) };
  } finally {
    testing.value = false;
  }
}

function onPreset(presetName: string) {
  settings.applyPreset(presetName);
}

function onThemeChange(th: "light" | "dark" | "system") {
  app.applyTheme(th);
}

const shortcuts = [
  { keys: "⌘ + K", desc: t('settings.shortcut_search') },
  { keys: "⌘ + ⇧ + Space", desc: t('settings.shortcut_toggle') },
  { keys: "↑ ↓", desc: t('settings.shortcut_nav') },
  { keys: "Enter", desc: t('settings.shortcut_open') },
  { keys: "Esc", desc: t('settings.shortcut_esc') },
  { keys: "⌘ + B", desc: t('settings.shortcut_chat') },
];
</script>

<template>
  <Teleport to="body">
    <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/30 backdrop-blur-sm">
      <div
        ref="root"
        class="flex max-h-[80vh] w-[680px] max-w-[94vw] flex-col rounded-xl border border-border bg-background shadow-2xl animate-[fade-in_0.15s_ease-out]"
      >
        <div class="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 class="text-sm font-semibold">{{ t('settings.title') }}</h2>
          <button
            class="rounded-md p-1 text-muted-foreground hover:bg-muted transition-colors"
            :aria-label="t('settings.close')"
            @click="emit('close')"
          >
            <X class="size-4" />
          </button>
        </div>

        <div class="flex flex-1 overflow-hidden">
          <aside class="w-40 shrink-0 border-r border-border bg-muted/20 p-2 space-y-0.5">
            <button
              v-for="tab in tabs" :key="tab.id"
              :class="[
                'flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-xs font-medium transition-colors text-left',
                currentTab === tab.id
                  ? 'bg-accent/10 text-accent'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground',
              ]"
              @click="currentTab = tab.id"
            >
              <component :is="tab.icon" class="size-4 shrink-0" />
              {{ t(tab.label) }}
            </button>
          </aside>

          <div class="flex-1 overflow-y-auto p-5">
            <div v-if="currentTab === 'provider'" class="space-y-4">
              <div>
                <label class="mb-1 block text-xs font-medium text-muted-foreground/80">{{ t('settings.presets') }}</label>
                <div class="flex flex-wrap gap-1">
                  <button
                    v-for="name in providerPresets" :key="name"
                    :class="[
                      'rounded-md px-2.5 py-1 text-xs font-medium transition-all',
                      settings.config.provider === name
                        ? 'bg-accent text-accent-foreground shadow-xs'
                        : 'bg-muted/70 text-muted-foreground hover:bg-muted hover:text-foreground'
                    ]"
                    @click="onPreset(name)"
                  >
                    {{ name }}
                  </button>
                </div>
              </div>

              <div class="space-y-3">
                <div>
                  <label class="mb-1 block text-xs font-medium text-muted-foreground/80">{{ t('settings.base_url') }}</label>
                  <input
                    v-model="settings.config.base_url"
                    class="w-full rounded-md border border-border/60 bg-background px-2.5 py-1.5 text-sm outline-none transition-colors focus:border-accent focus:ring-1 focus:ring-accent/20"
                    placeholder="https://api.openai.com/v1"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs font-medium text-muted-foreground/80">{{ t('settings.api_key') }}</label>
                  <div class="flex items-center gap-1.5">
                    <input
                      v-model="settings.config.api_key" :type="showKey ? 'text' : 'password'"
                      class="flex-1 rounded-md border border-border/60 bg-background px-2.5 py-1.5 text-sm outline-none transition-colors focus:border-accent focus:ring-1 focus:ring-accent/20"
                      placeholder="sk-..." autocomplete="off"
                    />
                    <button
                      class="rounded-md border border-border/60 bg-muted/60 p-1.5 text-muted-foreground hover:bg-muted transition-colors shrink-0"
                      :title="showKey ? t('settings.hide_key') : t('settings.show_key')"
                      @click="showKey = !showKey"
                    >
                      <Eye v-if="!showKey" class="size-3.5" />
                      <EyeOff v-else class="size-3.5" />
                    </button>
                  </div>
                </div>
                <div>
                  <label class="mb-1 block text-xs font-medium text-muted-foreground/80">{{ t('settings.model') }}</label>
                  <input
                    v-model="settings.config.model"
                    class="w-full rounded-md border border-border/60 bg-background px-2.5 py-1.5 text-sm outline-none transition-colors focus:border-accent focus:ring-1 focus:ring-accent/20"
                    placeholder="gpt-4o-mini"
                  />
                </div>
              </div>

              <div class="flex items-center gap-3">
                <button
                  :disabled="testing"
                  class="inline-flex items-center gap-1.5 rounded-md border border-border/60 bg-muted/60 px-2.5 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors disabled:opacity-50"
                  @click="onTest"
                >
                  <Loader2 v-if="testing" class="size-3 animate-spin" />
                  {{ t('settings.test') }}
                </button>
                <div
                  v-if="testResult"
                  :class="[
                    'inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium',
                    testResult.ok
                      ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/30 dark:text-emerald-400'
                      : 'bg-red-50 text-red-700 dark:bg-red-950/30 dark:text-red-400'
                  ]"
                >
                  <CheckCircle2 v-if="testResult.ok" class="size-3.5 shrink-0" />
                  <AlertCircle v-else class="size-3.5 shrink-0" />
                  <span class="truncate max-w-[280px]" :title="testResult.message">{{ testResult.message }}</span>
                </div>
              </div>
            </div>

            <div v-if="currentTab === 'indexing'" class="space-y-3">
              <div class="flex items-center gap-2">
                <span class="text-xs text-muted-foreground/70">{{ t('settings.index_status') }}:</span>
                <span
                  v-if="indexing"
                  class="inline-flex items-center gap-1.5 rounded-full bg-emerald-50 px-2.5 py-0.5 text-xs font-medium text-emerald-700 dark:bg-emerald-950/30 dark:text-emerald-400"
                >
                  <span class="size-1.5 rounded-full bg-emerald-500 animate-pulse" />
                  {{ t('settings.index_running') }}
                </span>
                <span
                  v-else-if="paused"
                  class="inline-flex items-center gap-1.5 rounded-full bg-yellow-50 px-2.5 py-0.5 text-xs font-medium text-yellow-700 dark:bg-yellow-950/30 dark:text-yellow-400"
                >
                  <span class="size-1.5 rounded-full bg-yellow-500" />
                  {{ t('settings.index_paused') }}
                </span>
                <span
                  v-else
                  class="inline-flex items-center gap-1.5 rounded-full bg-muted/60 px-2.5 py-0.5 text-xs font-medium text-muted-foreground"
                >
                  <span class="size-1.5 rounded-full bg-muted-foreground/40" />
                  {{ t('settings.index_idle') }}
                </span>
              </div>

              <div class="flex gap-1.5">
                <button
                  v-if="!indexing && !paused"
                  class="rounded-md bg-accent px-2.5 py-1.5 text-xs font-medium text-accent-foreground shadow-xs hover:brightness-110 transition-all"
                  @click="onStartIndex"
                >
                  {{ t('settings.index_start') }}
                </button>
                <button
                  v-if="indexing && !paused"
                  class="rounded-md border border-border/60 bg-background px-2.5 py-1.5 text-xs font-medium hover:bg-muted/50 transition-colors"
                  @click="onPauseIndex"
                >
                  {{ t('settings.index_pause') }}
                </button>
                <button
                  v-if="paused"
                  class="rounded-md bg-accent px-2.5 py-1.5 text-xs font-medium text-accent-foreground shadow-xs hover:brightness-110 transition-all"
                  @click="onResumeIndex"
                >
                  {{ t('settings.index_resume') }}
                </button>
                <button
                  v-if="indexing || paused"
                  class="rounded-md bg-destructive/80 px-2.5 py-1.5 text-xs font-medium text-white hover:bg-destructive transition-colors"
                  @click="onCancelIndex"
                >
                  {{ t('settings.index_cancel') }}
                </button>
              </div>

              <div v-if="indexedRoots.length" class="space-y-1">
                <div class="text-xs font-medium text-muted-foreground/70">{{ t('settings.index_dirs') }}:</div>
                <div
                  v-for="r in indexedRoots" :key="r"
                  class="rounded-md bg-muted/30 px-2.5 py-1 font-mono text-[11px] text-muted-foreground/70 truncate"
                >
                  {{ r }}
                </div>
              </div>
            </div>

            <div v-if="currentTab === 'appearance'" class="space-y-6">
              <section class="space-y-3">
                <h3 class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
                  <Palette class="size-3.5" />
                  {{ t('settings.theme') }}
                </h3>
                <div class="inline-flex rounded-lg border border-border/60 bg-muted/30 p-0.5">
                  <button
                    v-for="th in (['light', 'dark', 'system'] as const)" :key="th"
                    :class="[
                      'inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-all',
                      app.theme === th
                        ? 'bg-background text-foreground shadow-xs'
                        : 'text-muted-foreground hover:text-foreground'
                    ]"
                    @click="onThemeChange(th)"
                  >
                    <Sun v-if="th === 'light'" class="size-3.5" />
                    <Moon v-else-if="th === 'dark'" class="size-3.5" />
                    <Monitor v-else class="size-3.5" />
                    {{ t('settings.theme_' + th) }}
                  </button>
                </div>
              </section>

              <section class="space-y-3">
                <h3 class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
                  <Languages class="size-3.5" />
                  {{ t('settings.language') }}
                </h3>
                <select
                  v-model="app.locale"
                  class="w-full rounded-md border border-border/60 bg-background px-2.5 py-1.5 text-sm outline-none transition-colors focus:border-accent focus:ring-1 focus:ring-accent/20"
                >
                  <option value="zh-CN">{{ t('settings.lang_zh') }}</option>
                  <option value="en">{{ t('settings.lang_en') }}</option>
                </select>
              </section>
            </div>

            <div v-if="currentTab === 'shortcuts'" class="space-y-3">
              <div class="divide-y divide-border/50 rounded-lg border border-border/50 bg-background/80">
                <div
                  v-for="(s, i) in shortcuts" :key="s.keys"
                  class="flex items-center justify-between px-3 py-2 text-xs"
                  :class="i % 2 === 1 ? 'bg-muted/20' : ''"
                >
                  <span class="text-muted-foreground/80">{{ s.desc }}</span>
                  <kbd class="rounded-md bg-muted/70 px-2 py-0.5 font-mono text-[10px] tracking-wider text-foreground/60 border border-border/40">{{ s.keys }}</kbd>
                </div>
              </div>
            </div>

            <div v-if="currentTab === 'ignore'" class="space-y-3">
              <p class="text-[11px] leading-relaxed text-muted-foreground/70">
                {{ t('settings.ignore_desc') }}
              </p>
              <textarea
                v-model="ignoreRules"
                class="w-full rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs font-mono outline-none transition-colors focus:border-accent"
                rows="6"
                placeholder="*.log&#10;tmp/&#10;.temp"
              ></textarea>
              <div class="flex justify-end">
                <button
                  class="rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-foreground shadow-xs hover:brightness-110 transition-all"
                  @click="onSaveIgnoreRules"
                >
                  {{ t('settings.ignore_save') }}
                </button>
              </div>
            </div>

            <div v-if="currentTab === 'privacy'" class="space-y-3">
              <div class="space-y-2 text-[12px] leading-relaxed text-muted-foreground/80">
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/20" />
                  <span>{{ t('settings.privacy_local') }}</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/20" />
                  <span>{{ t('settings.privacy_key') }}</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/20" />
                  <span>{{ t('settings.privacy_telemetry') }}</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/20" />
                  <span>{{ t('settings.privacy_config') }}</span>
                </div>
              </div>
            </div>

            <div v-if="currentTab === 'stats'" class="space-y-3">
              <div class="grid grid-cols-3 gap-3">
                <div class="rounded-lg border border-border/50 bg-muted/20 p-4 text-center">
                  <FileText class="mx-auto mb-2 size-6 text-muted-foreground/40" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.files }}</div>
                  <div class="mt-1 text-[11px] text-muted-foreground/60">{{ t('settings.stats_files') }}</div>
                </div>
                <div class="rounded-lg border border-border/50 bg-muted/20 p-4 text-center">
                  <Hash class="mx-auto mb-2 size-6 text-muted-foreground/40" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.chunks }}</div>
                  <div class="mt-1 text-[11px] text-muted-foreground/60">{{ t('settings.stats_chunks') }}</div>
                </div>
                <div class="rounded-lg border border-border/50 bg-muted/20 p-4 text-center">
                  <Tags class="mx-auto mb-2 size-6 text-muted-foreground/40" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.tags }}</div>
                  <div class="mt-1 text-[11px] text-muted-foreground/60">{{ t('settings.stats_tags') }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="flex items-center justify-end gap-2 border-t border-border px-5 py-3">
          <button
            class="rounded-md border border-border/60 bg-background px-4 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted/50 transition-colors"
            @click="emit('close')"
          >
            {{ t('settings.cancel') }}
          </button>
          <button
            :disabled="saving"
            class="inline-flex items-center gap-1.5 rounded-md bg-accent px-4 py-1.5 text-xs font-medium text-accent-foreground shadow-xs hover:brightness-110 transition-all disabled:opacity-50"
            @click="onSave"
          >
            <Loader2 v-if="saving" class="size-3.5 animate-spin" />
            {{ t('settings.save') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
