<script setup lang="ts">
import { onMounted, ref } from "vue";
import { X, Eye, EyeOff, Loader2, CheckCircle2, AlertCircle } from "@lucide/vue";
import { onClickOutside, useMagicKeys, whenever } from "@vueuse/core";
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

onClickOutside(root, () => emit("close"));
const keys = useMagicKeys();
whenever(keys.escape, () => emit("close"));

onMounted(() => settings.load());

async function onSave() {
  saving.value = true;
  try {
    await settings.save();
    toast.push("设置已保存", "success");
    emit("close");
  } catch (e) {
    toast.push("保存失败: " + String(e), "error");
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
            @click="emit('close')"
            class="rounded-md p-1 text-muted-foreground hover:bg-muted"
            aria-label="关闭"
          >
            <X class="size-4" />
          </button>
        </div>

        <div class="flex-1 overflow-auto p-5">
          <section class="mb-5">
            <h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              LLM Provider
            </h3>

            <div class="mb-3 flex flex-wrap gap-1.5">
              <button
                v-for="name in providerPresets"
                :key="name"
                @click="onPreset(name)"
                :class="[
                  'rounded-md px-2.5 py-1 text-xs transition',
                  settings.config.provider === name
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted hover:bg-muted/80',
                ]"
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
                :type="showKey ? 'text' : 'password'"
                v-model="settings.config.api_key"
                class="flex-1 rounded-md border border-border bg-background px-3 py-1.5 text-sm outline-none focus:border-primary"
                placeholder="sk-..."
                autocomplete="off"
              />
              <button
                @click="showKey = !showKey"
                class="rounded-md bg-muted p-1.5 text-muted-foreground hover:bg-muted/80"
                :title="showKey ? '隐藏' : '显示'"
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
                @click="onTest"
                :disabled="testing"
                class="flex items-center gap-1 rounded-md bg-muted px-3 py-1.5 text-xs hover:bg-muted/80 disabled:opacity-50"
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

          <section class="rounded-md bg-muted/40 p-3 text-[11px] leading-relaxed text-muted-foreground">
            <p class="mb-1.5 font-medium text-foreground">提示</p>
            <p class="mb-1">• 所有配置存储在本地 SQLite，不上传任何服务器</p>
            <p class="mb-1">• 兼容 OpenAI 协议的 Provider 都可用：OpenAI / DeepSeek / 智谱 / Moonshot / 通义千问</p>
            <p>• 想用本地 LLM？后续版本会支持 Ollama</p>
          </section>
        </div>

        <div class="flex justify-end gap-2 border-t border-border px-5 py-3">
          <button
            @click="emit('close')"
            class="rounded-md bg-muted px-3 py-1.5 text-xs hover:bg-muted/80"
          >
            取消
          </button>
          <button
            @click="onSave"
            :disabled="saving"
            class="flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:opacity-90 disabled:opacity-50"
          >
            <Loader2 v-if="saving" class="size-3 animate-spin" />
            保存
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
