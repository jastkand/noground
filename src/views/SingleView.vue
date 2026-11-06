<script setup lang="ts">
import { computed } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import DropZone from '../components/DropZone.vue'
import CompareSlider from '../components/CompareSlider.vue'
import ProgressBar from '../components/ProgressBar.vue'
import { useApp } from '../store'
import { api } from '../api'

const store = useApp()

const progressPct = computed(() =>
  store.job.progress && store.job.progress.total > 0
    ? (store.job.progress.index / store.job.progress.total) * 100
    : null,
)

const jobLabel = computed(() => {
  const p = store.job.progress
  if (!p) return 'working…'
  return p.total > 1 ? `${p.index + 1} of ${p.total} · ${p.stage}` : `${p.stage}…`
})

async function chooseImages() {
  const picked = await open({
    multiple: true,
    filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'tif', 'tiff', 'bmp', 'gif'] }],
  })
  if (!picked) return
  const paths = Array.isArray(picked) ? picked : [picked]
  if (paths.length > 1) {
    store.route = 'batch'
  }
  await api.processPaths(paths)
}

async function chooseFolder() {
  const picked = await open({ directory: true })
  if (typeof picked === 'string') {
    store.route = 'batch'
    await api.processPaths([picked])
  }
}

async function cancel() {
  await api.cancelJob()
}

function startOver() {
  store.single.input = null
  store.single.output = null
}
</script>

<template>
  <div class="view">
    <header>
      <h1>Remove background</h1>
      <p class="muted">Runs fully offline — nothing leaves this Mac.</p>
    </header>

    <DropZone v-if="!store.single.output && !store.job.running" />

    <div v-if="store.job.running" class="panel running">
      <ProgressBar :value="progressPct" :label="jobLabel" />
      <button class="danger" @click="cancel">Cancel</button>
    </div>

    <template v-else-if="store.single.output">
      <CompareSlider :before="store.single.input!" :after="store.single.output" />
      <div class="actions">
        <button @click="revealItemInDir(store.single.output!)">Show in Finder</button>
        <button primary @click="startOver">Start over</button>
      </div>
      <p class="path muted">{{ store.single.output }}</p>
    </template>

    <div v-if="!store.single.output && !store.job.running" class="actions">
      <button @click="chooseImages">Choose images…</button>
      <button @click="chooseFolder">Choose folder…</button>
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

.actions {
  display: flex;
  gap: 10px;
}

.path {
  margin: 0;
  font-size: 11px;
  word-break: break-all;
  user-select: text;
}
</style>
