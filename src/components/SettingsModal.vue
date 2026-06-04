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

const stats = reactive({ files: 0, chunks: 0, tags: 0 });
const indexedRoots = ref<string[]>([]);
const indexing = ref(false);
const paused = ref(false);

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
          class="flex max-h-[85vh] w-[600px] max-w-[90vw] flex-col rounded-2xl border border-border bg-background shadow-2xl"
        >
          <div class="flex items-center justify-between border-b border-border px-6 py-4">
            <h2 class="text-base font-semibold">{{ t('settings.title') }}</h2>
            <button
              class="rounded-lg p-1.5 text-muted-foreground hover:bg-muted transition-colors"
              :aria-label="t('settings.close')"
              @click="emit('close')"
            >
              <X class="size-4" />
            </button>
          </div>

          <div class="flex-1 overflow-y-auto space-y-5 p-6">
            <!-- LLM Provider -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <HardDrive class="size-3.5" />
                {{ t('settings.llm_provider') }}
              </h3>

              <label class="mb-1.5 block text-xs font-medium text-muted-foreground">{{ t('settings.presets') }}</label>
              <div class="mb-4 flex flex-wrap gap-1.5">
                <button
                  v-for="name in providerPresets" :key="name"
                  :class="[
                    'rounded-lg px-3 py-1.5 text-xs font-medium transition-all',
                    settings.config.provider === name
                      ? 'bg-primary text-primary-foreground shadow-sm'
                      : 'bg-muted text-muted-foreground hover:bg-muted/80 hover:text-foreground'
                  ]"
                  @click="onPreset(name)"
                >
                  {{ name }}
                </button>
              </div>

              <div class="space-y-3">
                <div>
                  <label class="mb-1.5 block text-xs font-medium text-muted-foreground">{{ t('settings.base_url') }}</label>
                  <input
                    v-model="settings.config.base_url"
                    class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/20"
                    placeholder="https://api.openai.com/v1"
                  />
                </div>
                <div>
                  <label class="mb-1.5 block text-xs font-medium text-muted-foreground">{{ t('settings.api_key') }}</label>
                  <div class="flex items-center gap-1.5">
                    <input
                      v-model="settings.config.api_key" :type="showKey ? 'text' : 'password'"
                      class="flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/20"
                      placeholder="sk-..." autocomplete="off"
                    />
                    <button
                      class="rounded-lg border border-border bg-muted p-2 text-muted-foreground hover:bg-muted/80 transition-colors shrink-0"
                      :title="showKey ? t('settings.hide_key') : t('settings.show_key')"
                      @click="showKey = !showKey"
                    >
                      <Eye v-if="!showKey" class="size-4" />
                      <EyeOff v-else class="size-4" />
                    </button>
                  </div>
                </div>
                <div>
                  <label class="mb-1.5 block text-xs font-medium text-muted-foreground">{{ t('settings.model') }}</label>
                  <input
                    v-model="settings.config.model"
                    class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/20"
                    placeholder="gpt-4o-mini"
                  />
                </div>
              </div>

              <div class="mt-4 flex items-center gap-3">
                <button
                  :disabled="testing"
                  class="inline-flex items-center gap-1.5 rounded-lg border border-border bg-muted px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted/80 hover:text-foreground transition-colors disabled:opacity-50"
                  @click="onTest"
                >
                  <Loader2 v-if="testing" class="size-3 animate-spin" />
                  {{ t('settings.test') }}
                </button>
                <div
                  v-if="testResult"
                  :class="[
                    'inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium',
                    testResult.ok
                      ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/30 dark:text-emerald-400'
                      : 'bg-red-50 text-red-700 dark:bg-red-950/30 dark:text-red-400'
                  ]"
                >
                  <CheckCircle2 v-if="testResult.ok" class="size-3.5 shrink-0" />
                  <AlertCircle v-else class="size-3.5 shrink-0" />
                  <span class="truncate max-w-[300px]" :title="testResult.message">{{ testResult.message }}</span>
                </div>
              </div>
            </section>

            <!-- 索引管理 -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <FolderTree class="size-3.5" />
                {{ t('settings.indexing') }}
              </h3>

              <div class="mb-3 flex items-center gap-2">
                <span class="text-xs text-muted-foreground">{{ t('settings.index_status') }}:</span>
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
                  class="inline-flex items-center gap-1.5 rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground"
                >
                  <span class="size-1.5 rounded-full bg-muted-foreground/50" />
                  {{ t('settings.index_idle') }}
                </span>
              </div>

              <div class="mb-3 flex gap-1.5">
                <button
                  v-if="!indexing && !paused"
                  class="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity"
                  @click="onStartIndex"
                >
                  {{ t('settings.index_start') }}
                </button>
                <button
                  v-if="indexing && !paused"
                  class="rounded-lg border border-border bg-background px-3 py-1.5 text-xs font-medium hover:bg-muted/50 transition-colors"
                  @click="onPauseIndex"
                >
                  {{ t('settings.index_pause') }}
                </button>
                <button
                  v-if="paused"
                  class="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity"
                  @click="onResumeIndex"
                >
                  {{ t('settings.index_resume') }}
                </button>
                <button
                  v-if="indexing || paused"
                  class="rounded-lg bg-red-500/90 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-500 transition-colors"
                  @click="onCancelIndex"
                >
                  {{ t('settings.index_cancel') }}
                </button>
              </div>

              <div v-if="indexedRoots.length" class="space-y-1">
                <div class="text-xs font-medium text-muted-foreground">{{ t('settings.index_dirs') }}:</div>
                <div
                  v-for="r in indexedRoots" :key="r"
                  class="rounded-lg bg-muted/30 px-3 py-1.5 font-mono text-[11px] text-muted-foreground truncate"
                >
                  {{ r }}
                </div>
              </div>
            </section>

            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <FileText class="size-3.5" />
                {{ t('settings.stats') }}
              </h3>

              <div class="grid grid-cols-3 gap-3">
                <div class="rounded-xl border border-border bg-background p-3.5 text-center">
                  <FileText class="mx-auto mb-1.5 size-5 text-muted-foreground/60" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.files }}</div>
                  <div class="mt-0.5 text-[11px] text-muted-foreground">{{ t('settings.stats_files') }}</div>
                </div>
                <div class="rounded-xl border border-border bg-background p-3.5 text-center">
                  <Hash class="mx-auto mb-1.5 size-5 text-muted-foreground/60" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.chunks }}</div>
                  <div class="mt-0.5 text-[11px] text-muted-foreground">{{ t('settings.stats_chunks') }}</div>
                </div>
                <div class="rounded-xl border border-border bg-background p-3.5 text-center">
                  <Tags class="mx-auto mb-1.5 size-5 text-muted-foreground/60" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.tags }}</div>
                  <div class="mt-0.5 text-[11px] text-muted-foreground">{{ t('settings.stats_tags') }}</div>
                </div>
              </div>
            </section>

            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Palette class="size-3.5" />
                {{ t('settings.theme') }}
              </h3>

              <div class="inline-flex rounded-lg border border-border bg-background p-0.5">
                <button
                  v-for="th in (['light', 'dark', 'system'] as const)" :key="th"
                  :class="[
                    'inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-all',
                    app.theme === th
                      ? 'bg-primary text-primary-foreground shadow-sm'
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

            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Languages class="size-3.5" />
                {{ t('settings.language') }}
              </h3>

              <select
                v-model="app.locale"
                class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/20"
              >
                <option value="zh-CN">简体中文</option>
                <option value="en">English</option>
              </select>
            </section>

            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Keyboard class="size-3.5" />
                {{ t('settings.shortcuts') }}
              </h3>

              <div class="divide-y divide-border rounded-lg border border-border bg-background">
                <div
                  v-for="(s, i) in shortcuts" :key="s.keys"
                  class="flex items-center justify-between px-3 py-2 text-xs"
                  :class="i % 2 === 1 ? 'bg-muted/20' : ''"
                >
                  <span class="text-muted-foreground">{{ s.desc }}</span>
                  <kbd class="rounded-md bg-muted px-2 py-0.5 font-mono text-[10px] tracking-wider text-foreground/70 border border-border/50">{{ s.keys }}</kbd>
                </div>
              </div>
            </section>

            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <FileText class="size-3.5" />
                {{ t('settings.ignore_rules') }}
              </h3>
              <p class="mb-3 text-[11px] leading-relaxed text-muted-foreground">
                {{ t('settings.ignore_desc') }}
              </p>
              <textarea
                v-model="ignoreRules"
                class="w-full rounded-lg border border-border bg-background px-3 py-2 text-xs font-mono outline-none transition-colors focus:border-primary"
                rows="6"
                placeholder="*.log&#10;tmp/&#10;.temp"
              ></textarea>
              <div class="mt-3 flex justify-end">
                <button
                  class="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:opacity-90"
                  @click="onSaveIgnoreRules"
                >
                  {{ t('settings.ignore_save') }}
                </button>
              </div>
            </section>

            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Lock class="size-3.5" />
                {{ t('settings.privacy') }}
              </h3>

              <div class="space-y-2 text-[12px] leading-relaxed text-muted-foreground">
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>{{ t('settings.privacy_local') }}</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>{{ t('settings.privacy_key') }}</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>{{ t('settings.privacy_telemetry') }}</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>{{ t('settings.privacy_config') }}</span>
                </div>
              </div>
            </section>
          </div>

          <div class="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
            <button
              class="rounded-lg border border-border bg-background px-4 py-2 text-xs font-medium text-muted-foreground hover:bg-muted/50 hover:text-foreground transition-colors"
              @click="emit('close')"
            >
              {{ t('settings.cancel') }}
            </button>
            <button
              :disabled="saving"
              class="inline-flex items-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-50"
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

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
</style>
