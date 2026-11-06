<script setup lang="ts">
defineProps<{ value: number | null; label?: string }>()
</script>

<template>
  <div class="wrap">
    <div class="bar">
      <div
        class="fill"
        :class="{ indeterminate: value === null }"
        :style="{ width: value === null ? '100%' : `${value}%` }"
      ></div>
    </div>
    <span v-if="label" class="label muted">{{ label }}</span>
  </div>
</template>

<style scoped>
.wrap {
  display: flex;
  align-items: center;
  gap: 12px;
}

.bar {
  flex: 1;
  height: 8px;
  background: var(--panel-2);
  border-radius: 4px;
  overflow: hidden;
}

.fill {
  height: 100%;
  background: var(--accent);
  border-radius: 4px;
  transition: width 0.2s;
}

.fill.indeterminate {
  width: 100%;
  animation: pulse 1.2s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 0.4;
  }
  50% {
    opacity: 1;
  }
}

.label {
  font-size: 12px;
  white-space: nowrap;
}
</style>
