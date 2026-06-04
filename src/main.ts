import { createApp } from "vue";
import { createPinia } from "pinia";
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";
import { VueQueryPlugin } from "@tanstack/vue-query";
import App from "./App.vue";
import { useAppStore } from "./stores/app";
import { useToastStore } from "./stores/toast";
import "./styles.css";

const app = createApp(App);

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);
app.use(pinia);
app.use(VueQueryPlugin);

app.config.errorHandler = (err) => {
  console.error("[Vue error]", err);
  try {
    const toast = useToastStore();
    toast.push("发生错误: " + String(err), "error", { durationMs: 5000 });
  } catch {
    /* ignore */
  }
};

window.addEventListener("unhandledrejection", (e) => {
  console.error("[unhandled rejection]", e.reason);
});

app.mount("#app");

const store = useAppStore();
store.bootstrap();
