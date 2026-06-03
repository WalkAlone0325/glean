import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ProviderConfig {
  provider: string;
  base_url: string;
  api_key: string;
  model: string;
}

const DEFAULTS: ProviderConfig = {
  provider: "openai",
  base_url: "https://api.openai.com/v1",
  api_key: "",
  model: "gpt-4o-mini",
};

const PRESETS: Record<string, Partial<ProviderConfig>> = {
  openai: { base_url: "https://api.openai.com/v1", model: "gpt-4o-mini" },
  deepseek: { base_url: "https://api.deepseek.com/v1", model: "deepseek-chat" },
  zhipu: { base_url: "https://open.bigmodel.cn/api/paas/v4", model: "glm-4-flash" },
  moonshot: { base_url: "https://api.moonshot.cn/v1", model: "moonshot-v1-8k" },
  ali: { base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1", model: "qwen-turbo" },
};

export const useSettingsStore = defineStore("settings", () => {
  const config = ref<ProviderConfig>({ ...DEFAULTS });
  const loaded = ref(false);

  async function load() {
    try {
      const cfg = await invoke<ProviderConfig>("get_provider_config");
      config.value = { ...DEFAULTS, ...cfg };
    } catch (e) {
      console.warn("failed to load provider config:", e);
    } finally {
      loaded.value = true;
    }
  }

  async function save(): Promise<void> {
    await invoke("set_setting", { key: "llm.provider", value: config.value.provider });
    await invoke("set_setting", { key: "llm.base_url", value: config.value.base_url });
    await invoke("set_setting", { key: "llm.api_key", value: config.value.api_key });
    await invoke("set_setting", { key: "llm.model", value: config.value.model });
  }

  async function testConnection(): Promise<string> {
    return await invoke<string>("test_llm");
  }

  function applyPreset(name: string) {
    const preset = PRESETS[name];
    if (preset) {
      config.value = { ...config.value, provider: name, ...preset };
    }
  }

  return { config, loaded, load, save, testConnection, applyPreset };
});

export const providerPresets = Object.keys(PRESETS);
