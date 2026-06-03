import { createApp } from "vue";
import { createPinia } from "pinia";
import piniaPluginPersistedstate from "pinia-plugin-persistedstate";
import { VueQueryPlugin } from "@tanstack/vue-query";
import App from "./App.vue";
import { useAppStore } from "./stores/app";
import "./styles.css";

const app = createApp(App);

const pinia = createPinia();
pinia.use(piniaPluginPersistedstate);
app.use(pinia);
app.use(VueQueryPlugin);

app.mount("#app");

const store = useAppStore();
store.bootstrap();
