<script setup lang="ts">
import type { BatchFile } from '../store'

defineProps<{ file: BatchFile }>()
</script>

<template>
  <div class="row" :title="file.message">
    <span class="icon" :class="file.status">
      <template v-if="file.status === 'done'">✓</template>
      <template v-else-if="file.status === 'skipped'">»</template>
      <template v-else-if="file.status === 'failed'">✗</template>
      <template v-else>•</template>
    </span>
    <span class="name">{{ file.name }}</span>
    <span v-if="file.message" class="msg muted">{{ file.message }}</span>
  </div>
</template>

<style scoped>
.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 12px;
  border-bottom: 1px solid var(--border);
  font-size: 12px;
}

.row:last-child {
  border-bottom: none;
}

.icon {
  width: 16px;
  text-align: center;
  flex-shrink: 0;
}

.icon.done {
  color: var(--ok);
}

.icon.skipped {
  color: var(--muted);
}

.icon.failed {
  color: var(--danger);
}

.icon.pending {
  color: var(--muted);
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.msg {
  margin-left: auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 40%;
  font-size: 11px;
}
</style>
