<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useToastStore } from "../stores/toast";
import { CheckCircle2, AlertCircle, Info, AlertTriangle, X } from "@lucide/vue";

const { t } = useI18n();
const toast = useToastStore();
</script>

<template>
  <Teleport to="body">
    <div
      class="pointer-events-none fixed left-1/2 top-6 z-[100] flex -translate-x-1/2 flex-col items-center gap-2"
    >
      <transition-group name="toast" tag="div" class="flex flex-col items-center gap-2">
        <div
          v-for="t in toast.items"
          :key="t.id"
          :class="[
            'pointer-events-auto flex max-w-md items-start gap-2 rounded-md border px-3 py-2 text-sm shadow-lg backdrop-blur',
            t.kind === 'success'
              ? 'border-emerald-500/30 bg-emerald-500/15 text-emerald-700 dark:text-emerald-300'
              : t.kind === 'error'
                ? 'border-red-500/30 bg-red-500/15 text-red-700 dark:text-red-300'
                : t.kind === 'warning'
                  ? 'border-yellow-500/30 bg-yellow-500/15 text-yellow-700 dark:text-yellow-300'
                  : 'border-border bg-background/90 text-foreground',
          ]"
        >
          <CheckCircle2 v-if="t.kind === 'success'" class="mt-0.5 size-4 shrink-0" />
          <AlertCircle v-else-if="t.kind === 'error'" class="mt-0.5 size-4 shrink-0" />
          <AlertTriangle v-else-if="t.kind === 'warning'" class="mt-0.5 size-4 shrink-0" />
          <Info v-else class="mt-0.5 size-4 shrink-0" />
          <div class="flex-1 break-words">
            {{ t.message }}
            <button
              v-if="t.action"
              class="ml-2 underline underline-offset-2 hover:opacity-80"
              @click="t.action.onClick(); toast.remove(t.id)"
            >
              {{ t.action.label }}
            </button>
          </div>
          <button
            class="ml-1 rounded p-0.5 opacity-60 hover:bg-black/10 hover:opacity-100 dark:hover:bg-white/10"
            :aria-label="t('settings.close')"
            @click="toast.remove(t.id)"
          >
            <X class="size-3.5" />
          </button>
        </div>
      </transition-group>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: all 0.22s ease;
}
.toast-enter-from {
  opacity: 0;
  transform: translateY(-8px);
}
.toast-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
