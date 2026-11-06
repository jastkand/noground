<script setup lang="ts">
import { computed } from 'vue'
import { useApp } from '../store'

const props = defineProps<{ compact?: boolean }>()
const store = useApp()

const hint = computed(() =>
  props.compact ? 'Drop an image here' : 'Drop an image or a folder of images here',
)
</script>

<template>
  <div class="dropzone panel" :class="{ active: store.dragActive, compact: props.compact }">
    <svg viewBox="0 0 24 24" width="42" height="42" fill="none" stroke="currentColor"
         stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
      <path d="M12 16V4m0 0-4 4m4-4 4 4" />
      <path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3" />
    </svg>
    <p>{{ hint }}</p>
  </div>
</template>

<style scoped>
.dropzone {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 60px 24px;
  border-style: dashed;
  border-width: 2px;
  color: var(--muted);
  transition: border-color 0.15s, background 0.15s;
}

.dropzone.compact {
  padding: 28px 16px;
}

.dropzone p {
  margin: 0;
  font-size: 14px;
}

.dropzone.active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 8%, var(--panel));
  color: var(--text);
}
</style>
