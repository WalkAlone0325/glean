<script setup lang="ts">
import { onMounted, ref, reactive } from "vue";
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
    toast.success("索引已启动");
  } catch (e) {
    toast.error("启动索引失败: " + String(e));
  }
}

async function onCancelIndex() {
  try {
    await invoke("cancel_indexing");
    indexing.value = false;
    paused.value = false;
  } catch (e) {
    toast.error("取消失败: " + String(e));
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
    toast.success("忽略规则已保存");
  } catch (e) {
    toast.error("保存忽略规则失败: " + String(e));
  }
}

async function onSave() {
  saving.value = true;
  try {
    await settings.save();
    toast.success("设置已保存");
    emit("close");
  } catch (e) {
    toast.error("保存失败: " + String(e));
  } finally {
    saving.value = false;
  }
}

async function onTest() {
  if (!settings.config.api_key) {
    testResult.value = { ok: false, message: "请先填入 API Key" };
    return;
  }
  testing.value = true;
  testResult.value = null;
  try {
    await settings.save();
    const reply = await settings.testConnection();
    testResult.value = { ok: true, message: `连通成功，模型回复: ${reply}` };
  } catch (e) {
    testResult.value = { ok: false, message: String(e) };
  } finally {
    testing.value = false;
  }
}

function onPreset(name: string) {
  settings.applyPreset(name);
}

function onThemeChange(t: "light" | "dark" | "system") {
  app.applyTheme(t);
}

const shortcuts = [
  { keys: "⌘ + K", desc: "打开文件搜索面板" },
  { keys: "⌘ + ⇧ + Space", desc: "显示/隐藏窗口" },
  { keys: "↑ ↓", desc: "搜索结果中移动选择" },
  { keys: "Enter", desc: "打开选中的文件 / 发送消息" },
  { keys: "Esc", desc: "关闭面板/弹窗" },
  { keys: "⌘ + B", desc: "切换聊天面板" },
];
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/30 backdrop-blur-sm">
        <div
          ref="root"
          class="flex max-h-[85vh] w-[600px] max-w-[90vw] flex-col rounded-2xl border border-border bg-background shadow-2xl"
        >
          <div class="flex items-center justify-between border-b border-border px-6 py-4">
            <h2 class="text-base font-semibold">设置</h2>
            <button
              class="rounded-lg p-1.5 text-muted-foreground hover:bg-muted transition-colors"
              aria-label="关闭"
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
                LLM Provider
              </h3>

              <label class="mb-1.5 block text-xs font-medium text-muted-foreground">预设方案</label>
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
                  <label class="mb-1.5 block text-xs font-medium text-muted-foreground">Base URL</label>
                  <input
                    v-model="settings.config.base_url"
                    class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/20"
                    placeholder="https://api.openai.com/v1"
                  />
                </div>
                <div>
                  <label class="mb-1.5 block text-xs font-medium text-muted-foreground">API Key</label>
                  <div class="flex items-center gap-1.5">
                    <input
                      v-model="settings.config.api_key" :type="showKey ? 'text' : 'password'"
                      class="flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/20"
                      placeholder="sk-..." autocomplete="off"
                    />
                    <button
                      class="rounded-lg border border-border bg-muted p-2 text-muted-foreground hover:bg-muted/80 transition-colors shrink-0"
                      :title="showKey ? '隐藏' : '显示'"
                      @click="showKey = !showKey"
                    >
                      <Eye v-if="!showKey" class="size-4" />
                      <EyeOff v-else class="size-4" />
                    </button>
                  </div>
                </div>
                <div>
                  <label class="mb-1.5 block text-xs font-medium text-muted-foreground">Model</label>
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
                  测试连通性
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
                索引管理
              </h3>

              <div class="mb-3 flex items-center gap-2">
                <span class="text-xs text-muted-foreground">状态：</span>
                <span
                  v-if="indexing"
                  class="inline-flex items-center gap-1.5 rounded-full bg-emerald-50 px-2.5 py-0.5 text-xs font-medium text-emerald-700 dark:bg-emerald-950/30 dark:text-emerald-400"
                >
                  <span class="size-1.5 rounded-full bg-emerald-500 animate-pulse" />
                  正在索引...
                </span>
                <span
                  v-else-if="paused"
                  class="inline-flex items-center gap-1.5 rounded-full bg-yellow-50 px-2.5 py-0.5 text-xs font-medium text-yellow-700 dark:bg-yellow-950/30 dark:text-yellow-400"
                >
                  <span class="size-1.5 rounded-full bg-yellow-500" />
                  已暂停
                </span>
                <span
                  v-else
                  class="inline-flex items-center gap-1.5 rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground"
                >
                  <span class="size-1.5 rounded-full bg-muted-foreground/50" />
                  空闲
                </span>
              </div>

              <div class="mb-3 flex gap-1.5">
                <button
                  v-if="!indexing && !paused"
                  class="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity"
                  @click="onStartIndex"
                >
                  开始索引
                </button>
                <button
                  v-if="indexing && !paused"
                  class="rounded-lg border border-border bg-background px-3 py-1.5 text-xs font-medium hover:bg-muted/50 transition-colors"
                  @click="onPauseIndex"
                >
                  暂停
                </button>
                <button
                  v-if="paused"
                  class="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity"
                  @click="onResumeIndex"
                >
                  恢复
                </button>
                <button
                  v-if="indexing || paused"
                  class="rounded-lg bg-red-500/90 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-500 transition-colors"
                  @click="onCancelIndex"
                >
                  取消
                </button>
              </div>

              <div v-if="indexedRoots.length" class="space-y-1">
                <div class="text-xs font-medium text-muted-foreground">已索引目录：</div>
                <div
                  v-for="r in indexedRoots" :key="r"
                  class="rounded-lg bg-muted/30 px-3 py-1.5 font-mono text-[11px] text-muted-foreground truncate"
                >
                  {{ r }}
                </div>
              </div>
            </section>

            <!-- 统计 -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <FileText class="size-3.5" />
                统计
              </h3>

              <div class="grid grid-cols-3 gap-3">
                <div class="rounded-xl border border-border bg-background p-3.5 text-center">
                  <FileText class="mx-auto mb-1.5 size-5 text-muted-foreground/60" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.files }}</div>
                  <div class="mt-0.5 text-[11px] text-muted-foreground">文件</div>
                </div>
                <div class="rounded-xl border border-border bg-background p-3.5 text-center">
                  <Hash class="mx-auto mb-1.5 size-5 text-muted-foreground/60" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.chunks }}</div>
                  <div class="mt-0.5 text-[11px] text-muted-foreground">分块</div>
                </div>
                <div class="rounded-xl border border-border bg-background p-3.5 text-center">
                  <Tags class="mx-auto mb-1.5 size-5 text-muted-foreground/60" />
                  <div class="text-xl font-bold tabular-nums">{{ stats.tags }}</div>
                  <div class="mt-0.5 text-[11px] text-muted-foreground">标签</div>
                </div>
              </div>
            </section>

            <!-- 主题 -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Palette class="size-3.5" />
                主题
              </h3>

              <div class="inline-flex rounded-lg border border-border bg-background p-0.5">
                <button
                  v-for="t in (['light', 'dark', 'system'] as const)" :key="t"
                  :class="[
                    'inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-all',
                    app.theme === t
                      ? 'bg-primary text-primary-foreground shadow-sm'
                      : 'text-muted-foreground hover:text-foreground'
                  ]"
                  @click="onThemeChange(t)"
                >
                  <Sun v-if="t === 'light'" class="size-3.5" />
                  <Moon v-else-if="t === 'dark'" class="size-3.5" />
                  <Monitor v-else class="size-3.5" />
                  {{ { light: '浅色', dark: '深色', system: '跟随系统' }[t] }}
                </button>
              </div>
            </section>

            <!-- 语言 -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Languages class="size-3.5" />
                语言
              </h3>

              <select
                v-model="app.locale"
                class="w-full rounded-lg border border-border bg-background px-3 py-2 text-sm outline-none transition-colors focus:border-primary focus:ring-1 focus:ring-primary/20"
              >
                <option value="zh-CN">简体中文</option>
                <option value="en">English</option>
              </select>
            </section>

            <!-- 快捷键 -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Keyboard class="size-3.5" />
                快捷键
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

            <!-- 忽略规则 -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <FileText class="size-3.5" />
                忽略规则
              </h3>
              <p class="mb-3 text-[11px] leading-relaxed text-muted-foreground">
                每行一个忽略模式，支持通配符（*?）。内置已忽略 .git / node_modules / .DS_Store 等。
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
                  保存忽略规则
                </button>
              </div>
            </section>

            <!-- 隐私 -->
            <section class="rounded-xl border border-border bg-muted/10 p-5">
              <h3 class="mb-4 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <Lock class="size-3.5" />
                隐私
              </h3>

              <div class="space-y-2 text-[12px] leading-relaxed text-muted-foreground">
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>所有文件索引数据存储在本地 SQLite，不上传任何服务器</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>API Key 仅用于调用你配置的 LLM Provider，不会传输给第三方</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>应用不会收集使用统计、崩溃报告或任何遥测数据</span>
                </div>
                <div class="flex items-start gap-2">
                  <span class="mt-0.5 block size-1.5 shrink-0 rounded-full bg-muted-foreground/30" />
                  <span>配置的索引目录和忽略规则仅保存在本地</span>
                </div>
              </div>
            </section>
          </div>

          <div class="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
            <button
              class="rounded-lg border border-border bg-background px-4 py-2 text-xs font-medium text-muted-foreground hover:bg-muted/50 hover:text-foreground transition-colors"
              @click="emit('close')"
            >
              取消
            </button>
            <button
              :disabled="saving"
              class="inline-flex items-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-50"
              @click="onSave"
            >
              <Loader2 v-if="saving" class="size-3.5 animate-spin" />
              保存
            </button>
          </div>
        </div>
      </div>
    </Transition>
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
