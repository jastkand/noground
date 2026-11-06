<script setup lang="ts">
import { computed } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { openPath } from '@tauri-apps/plugin-opener'
import DropZone from '../components/DropZone.vue'
import FileRow from '../components/FileRow.vue'
import ProgressBar from '../components/ProgressBar.vue'
import { useApp } from '../store'
import { api } from '../api'

const store = useApp()

const hasFiles = computed(() => store.batch.files.length > 0)

const progressPct = computed(() =>
  store.job.progress && store.job.progress.total > 0
    ? (store.job.progress.index / store.job.progress.total) * 100
    : null,
)

const header = computed(() => {
  const p = store.job.progress
  if (store.job.running && p) {
    return `${Math.min(p.index + 1, p.total)} of ${p.total}`
  }
  const done = store.batch.files.filter((f) => f.status === 'done').length
  const skipped = store.batch.files.filter((f) => f.status === 'skipped').length
  const failed = store.batch.files.filter((f) => f.status === 'failed').length
  return `${done} processed${skipped ? `, ${skipped} skipped` : ''}${failed ? `, ${failed} failed` : ''}`
})

async function chooseFolder() {
  const picked = await open({ directory: true })
  if (typeof picked === 'string') {
    store.batch.files = []
    await api.processPaths([picked])
  }
}

async function cancel() {
  await api.cancelJob()
}

function openOutDir() {
  if (store.batch.outDir) {
    void openPath(store.batch.outDir)
  }
}
</script>

<template>
  <div class="view">
    <header>
      <h1>Batch</h1>
      <p class="muted">
        {{ hasFiles ? header : 'Drop a folder — every image inside gets the background removed.' }}
      </p>
    </header>

    <DropZone v-if="!hasFiles && !store.job.running" />

    <div v-if="store.job.running" class="panel running">
      <ProgressBar :value="progressPct" :label="store.job.progress?.stage" />
      <button class="danger" @click="cancel">Cancel</button>
    </div>

    <div v-if="hasFiles" class="list panel">
      <FileRow v-for="file in store.batch.files" :key="file.path" :file="file" />
    </div>

    <div v-if="hasFiles && !store.job.running && store.batch.outDir" class="actions">
      <button @click="openOutDir">Open folder</button>
      <button @click="chooseFolder">Process another folder…</button>
    </div>
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
}

header h1 {
  margin: 0 0 2px;
  font-size: 20px;
}

.running {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 16px;
}

.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.actions {
  display: flex;
  gap: 10px;
}
</style>
