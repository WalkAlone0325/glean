<script setup lang="ts">
import { useToastStore } from "../stores/toast";
import { CheckCircle2, AlertCircle, Info } from "@lucide/vue";

const toast = useToastStore();
</script>

<template>
  <Teleport to="body">
    <div class="pointer-events-none fixed left-1/2 top-6 z-[100] flex -translate-x-1/2 flex-col items-center gap-2">
      <transition-group name="toast" tag="div" class="flex flex-col items-center gap-2">
        <div
          v-for="t in toast.items"
          :key="t.id"
          :class="[
            'pointer-events-auto flex items-center gap-2 rounded-md border px-3 py-2 text-sm shadow-lg backdrop-blur',
            t.kind === 'success'
              ? 'border-emerald-500/30 bg-emerald-500/15 text-emerald-700 dark:text-emerald-300'
              : t.kind === 'error'
                ? 'border-red-500/30 bg-red-500/15 text-red-700 dark:text-red-300'
                : 'border-border bg-background/90 text-foreground',
          ]"
        >
          <CheckCircle2 v-if="t.kind === 'success'" class="size-4" />
          <AlertCircle v-else-if="t.kind === 'error'" class="size-4" />
          <Info v-else class="size-4" />
          {{ t.message }}
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
