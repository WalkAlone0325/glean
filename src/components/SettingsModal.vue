<script setup lang="ts">
import { onMounted, ref, reactive } from "vue";
import { X, Eye, EyeOff, Loader2, CheckCircle2, AlertCircle, FolderTree, HardDrive } from "@lucide/vue";
import { onClickOutside, useMagicKeys, whenever } from "@vueuse/core";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore, providerPresets } from "../stores/settings";
import { useToastStore } from "../stores/toast";

const emit = defineEmits<{ close: [] }>();

const settings = useSettingsStore();
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
</script>

<template>
  <Teleport to="body">
    <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/30 backdrop-blur-sm">
      <div
        ref="root"
        class="flex w-[520px] max-w-[90vw] flex-col rounded-xl border border-border bg-background shadow-2xl"
      >
        <div class="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 class="text-sm font-semibold">设置</h2>
          <button
            class="rounded-md p-1 text-muted-foreground hover:bg-muted"
            aria-label="关闭"
            @click="emit('close')"
          >
            <X class="size-4" />
          </button>
        </div>

        <div class="flex-1 overflow-auto p-5 space-y-6">
          <!-- LLM Provider -->
          <section>
            <h3 class="mb-2 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              <HardDrive class="size-3.5" />
              LLM Provider
            </h3>
            <div class="mb-3 flex flex-wrap gap-1.5">
              <button
                v-for="name in providerPresets"
                :key="name"
                :class="[
                  'rounded-md px-2.5 py-1 text-xs transition',
                  settings.config.provider === name
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted hover:bg-muted/80',
                ]"
                @click="onPreset(name)"
              >
                {{ name }}
              </button>
            </div>
            <label class="mb-1 block text-xs text-muted-foreground">Base URL</label>
            <input
              v-model="settings.config.base_url"
              class="mb-3 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm outline-none focus:border-primary"
              placeholder="https://api.openai.com/v1"
            />
            <label class="mb-1 block text-xs text-muted-foreground">API Key</label>
            <div class="mb-3 flex items-center gap-1">
              <input
                v-model="settings.config.api_key"
                :type="showKey ? 'text' : 'password'"
                class="flex-1 rounded-md border border-border bg-background px-3 py-1.5 text-sm outline-none focus:border-primary"
                placeholder="sk-..."
                autocomplete="off"
              />
              <button
                class="rounded-md bg-muted p-1.5 text-muted-foreground hover:bg-muted/80"
                :title="showKey ? '隐藏' : '显示'"
                @click="showKey = !showKey"
              >
                <Eye v-if="!showKey" class="size-4" />
                <EyeOff v-else class="size-4" />
              </button>
            </div>
            <label class="mb-1 block text-xs text-muted-foreground">Model</label>
            <input
              v-model="settings.config.model"
              class="w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm outline-none focus:border-primary"
              placeholder="gpt-4o-mini"
            />
            <div class="mt-3 flex items-center gap-2">
              <button
                :disabled="testing"
                class="flex items-center gap-1 rounded-md bg-muted px-3 py-1.5 text-xs hover:bg-muted/80 disabled:opacity-50"
                @click="onTest"
              >
                <Loader2 v-if="testing" class="size-3 animate-spin" />
                测试连通性
              </button>
              <div
                v-if="testResult"
                :class="[
                  'flex items-center gap-1 text-xs',
                  testResult.ok ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-500',
                ]"
              >
                <CheckCircle2 v-if="testResult.ok" class="size-3.5" />
                <AlertCircle v-else class="size-3.5" />
                <span class="truncate" :title="testResult.message">{{ testResult.message }}</span>
              </div>
            </div>
          </section>

          <!-- 索引管理 -->
          <section>
            <h3 class="mb-2 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              <FolderTree class="size-3.5" />
              索引管理
            </h3>
            <div class="mb-2 space-y-1">
              <div class="flex items-center gap-2 text-xs">
                <span class="opacity-70">状态：</span>
                <span v-if="indexing" class="text-emerald-600 dark:text-emerald-400">正在索引...</span>
                <span v-else-if="paused" class="text-yellow-600">已暂停</span>
                <span v-else class="opacity-70">空闲</span>
              </div>
            </div>
            <div class="mb-3 flex gap-1.5">
              <button
                v-if="!indexing && !paused"
                class="rounded-md bg-primary px-2.5 py-1 text-xs text-primary-foreground hover:opacity-90"
                @click="onStartIndex"
              >
                开始索引
              </button>
              <button
                v-if="indexing && !paused"
                class="rounded-md bg-muted px-2.5 py-1 text-xs hover:bg-muted/80"
                @click="onPauseIndex"
              >
                暂停
              </button>
              <button
                v-if="paused"
                class="rounded-md bg-primary px-2.5 py-1 text-xs text-primary-foreground hover:opacity-90"
                @click="onResumeIndex"
              >
                恢复
              </button>
              <button
                v-if="indexing || paused"
                class="rounded-md bg-red-500/80 px-2.5 py-1 text-xs text-white hover:bg-red-500"
                @click="onCancelIndex"
              >
                取消
              </button>
            </div>
            <div v-if="indexedRoots.length" class="text-xs">
              <div class="mb-1 opacity-70">已索引目录：</div>
              <div v-for="r in indexedRoots" :key="r" class="rounded bg-muted/40 px-2 py-0.5 font-mono text-[10px]">
                {{ r }}
              </div>
            </div>
          </section>

          <!-- 统计 -->
          <section>
            <h3 class="mb-2 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              统计
            </h3>
            <div class="grid grid-cols-3 gap-2 text-center text-xs">
              <div class="rounded-md bg-muted/40 p-2">
                <div class="text-lg font-semibold">{{ stats.files }}</div>
                <div class="opacity-60">文件</div>
              </div>
              <div class="rounded-md bg-muted/40 p-2">
                <div class="text-lg font-semibold">{{ stats.chunks }}</div>
                <div class="opacity-60">分块</div>
              </div>
              <div class="rounded-md bg-muted/40 p-2">
                <div class="text-lg font-semibold">{{ stats.tags }}</div>
                <div class="opacity-60">标签</div>
              </div>
            </div>
          </section>

          <!-- 提示 -->
          <section class="rounded-md bg-muted/40 p-3 text-[11px] leading-relaxed text-muted-foreground">
            <p class="mb-1.5 font-medium text-foreground">提示</p>
            <p class="mb-1">• 所有配置存储在本地 SQLite，不上传任何服务器</p>
            <p class="mb-1">• 兼容 OpenAI 协议的 Provider 都可用：OpenAI / DeepSeek / 智谱 / Moonshot / 通义千问</p>
            <p>• 想用本地 LLM？后续版本会支持 Ollama</p>
          </section>
        </div>

        <div class="flex justify-end gap-2 border-t border-border px-5 py-3">
          <button
            class="rounded-md bg-muted px-3 py-1.5 text-xs hover:bg-muted/80"
            @click="emit('close')"
          >
            取消
          </button>
          <button
            :disabled="saving"
            class="flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:opacity-90 disabled:opacity-50"
            @click="onSave"
          >
            <Loader2 v-if="saving" class="size-3 animate-spin" />
            保存
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
