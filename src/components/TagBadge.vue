<script setup lang="ts">
import { computed } from "vue";
import { X } from "@lucide/vue";

const props = defineProps<{
  name: string;
  color?: string | null;
  removable?: boolean;
  small?: boolean;
}>();

const emit = defineEmits<{ remove: [] }>();

const TAG_COLORS: Record<string, string> = {
  red: "bg-red-100 text-red-700 border-red-200 dark:bg-red-900/30 dark:text-red-300",
  orange: "bg-orange-100 text-orange-700 border-orange-200 dark:bg-orange-900/30 dark:text-orange-300",
  yellow: "bg-yellow-100 text-yellow-700 border-yellow-200 dark:bg-yellow-900/30 dark:text-yellow-300",
  green: "bg-emerald-100 text-emerald-700 border-emerald-200 dark:bg-emerald-900/30 dark:text-emerald-300",
  blue: "bg-blue-100 text-blue-700 border-blue-200 dark:bg-blue-900/30 dark:text-blue-300",
  purple: "bg-purple-100 text-purple-700 border-purple-200 dark:bg-purple-900/30 dark:text-purple-300",
  pink: "bg-pink-100 text-pink-700 border-pink-200 dark:bg-pink-900/30 dark:text-pink-300",
  gray: "bg-zinc-100 text-zinc-700 border-zinc-200 dark:bg-zinc-800/30 dark:text-zinc-300",
};

const colorClass = computed(() => {
  if (props.color && TAG_COLORS[props.color]) return TAG_COLORS[props.color];
  return TAG_COLORS.gray;
});

const sizeClass = computed(() => (props.small ? "text-[10px] px-1.5 py-0.5" : "text-xs px-2 py-0.5"));
</script>

<template>
  <span
    :class="[
      'inline-flex items-center gap-0.5 rounded-full border font-medium leading-none',
      colorClass,
      sizeClass,
    ]"
  >
    {{ name }}
    <button
      v-if="removable"
      class="-mr-0.5 rounded-full p-0.5 hover:opacity-70"
      aria-label="移除标签"
      @click.stop="emit('remove')"
    >
      <X class="size-3" />
    </button>
  </span>
</template>
