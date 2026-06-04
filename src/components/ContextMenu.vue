<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";

defineProps<{
  x: number;
  y: number;
}>();

const emit = defineEmits<{ close: [] }>();
const root = ref<HTMLElement | null>(null);

function onDocumentClick(e: MouseEvent) {
  if (root.value && !root.value.contains(e.target as Node)) {
    emit("close");
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}

onMounted(() => {
  document.addEventListener("mousedown", onDocumentClick);
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("mousedown", onDocumentClick);
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div
      ref="root"
      :style="{
        position: 'fixed',
        left: x + 'px',
        top: y + 'px',
        zIndex: 1000,
        transform: 'translate(0, -100%)',
      }"
      class="min-w-[160px] rounded-md border border-border bg-background py-1 text-xs shadow-xl"
      @contextmenu.prevent
    >
      <slot />
    </div>
  </Teleport>
</template>
