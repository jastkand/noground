<script setup lang="ts">
import { onMounted } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { listen } from '@tauri-apps/api/event'
import Sidebar from './components/Sidebar.vue'
import SingleView from './views/SingleView.vue'
import BatchView from './views/BatchView.vue'
import SettingsView from './views/SettingsView.vue'
import { useApp } from './store'
import { api } from './api'
import type { JobDone, JobProgress, ModelDownloadProgress } from './types'

const store = useApp()

onMounted(async () => {
  store.settings = await api.getSettings().catch(() => null)
  store.version = await getVersion().catch(() => '')

  await getCurrentWebview().onDragDropEvent((e) => {
    if (e.payload.type === 'enter' || e.payload.type === 'over') {
      store.dragActive = true
    } else if (e.payload.type === 'leave') {
      store.dragActive = false
    } else if (e.payload.type === 'drop') {
      store.dragActive = false
      void handleDropped(e.payload.paths)
    }
  })

  await listen<JobProgress>('noground://job-progress', (e) => {
    const p = e.payload
    store.job.progress = p
    store.job.running = true
    if (p.total === 1 && store.route === 'single' && !store.single.input) {
      store.single.input = p.file
    }
    if (store.route === 'batch' && !store.batch.files.some((f) => f.path === p.file)) {
      store.batch.files.push({ path: p.file, name: fileName(p.file), status: 'pending' })
    }
  })

  await listen<JobDone>('noground://job-done', (e) => {
    const done = e.payload
    store.job.running = false
    store.job.progress = null
    if (store.route === 'single') {
      store.single.output = done.outputs[0]?.output ?? null
      if (!store.single.output) {
        const failure = done.failed[0]
        if (failure) store.error = failure.message
      }
    } else {
      const failedMap = new Map(done.failed.map((f) => [f.input, f.message]))
      const outputsMap = new Map(done.outputs.map((o) => [o.input, o.output]))
      for (const row of store.batch.files) {
        if (outputsMap.has(row.path)) {
          row.status = 'done'
        } else if (failedMap.has(row.path)) {
          row.status = 'failed'
          row.message = failedMap.get(row.path)
        } else if (done.skipped.includes(row.path)) {
          row.status = 'skipped'
        }
      }
      store.batch.outDir = done.outDir ?? store.batch.outDir
    }
  })

  await listen<{ message: string }>('noground://job-failed', (e) => {
    store.job.running = false
    store.job.progress = null
    if (e.payload.message !== 'cancelled') {
      store.error = e.payload.message
    }
  })

  await listen<ModelDownloadProgress>('noground://model-progress', async (e) => {
    const p = e.payload
    if (store.download.key !== p.key) {
      store.download.key = p.key
    }
    store.download.downloadedBytes = p.downloadedBytes
    store.download.totalBytes = p.totalBytes
    if (p.done) {
      store.models = await api.listModels().catch(() => store.models)
    }
  })

  await listen<{ key: string; message: string }>('noground://model-error', async (e) => {
    store.error = e.payload.message
    store.download.key = null
    store.models = await api.listModels().catch(() => store.models)
  })
})

function fileName(path: string): string {
  const idx = path.lastIndexOf('/')
  return idx >= 0 ? path.slice(idx + 1) : path
}

async function handleDropped(paths: string[]) {
  const entries = await api.inspectPaths(paths)
  const dir = entries.find((e) => e.isDir)
  if (dir || entries.length > 1) {
    store.route = 'batch'
    store.batch.files = []
    store.batch.outDir = null
  } else {
    store.route = 'single'
    store.single.input = entries[0]?.path ?? null
    store.single.output = null
    store.job.running = true
  }
  try {
    await api.processPaths(entries.map((e) => e.path))
  } catch (e) {
    store.job.running = false
    store.error = String(e)
  }
}

function dismissError() {
  store.error = null
}
</script>

<template>
  <div class="app" :class="{ 'drag-target': store.dragActive }">
    <Sidebar />
    <main class="content">
      <SingleView v-if="store.route === 'single'" />
      <BatchView v-else-if="store.route === 'batch'" />
      <SettingsView v-else />
    </main>
    <div v-if="store.error" class="error-banner">
      <span>{{ store.error }}</span>
      <button class="dismiss" aria-label="Dismiss" @click="dismissError">✕</button>
    </div>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  height: 100%;
  position: relative;
}

.content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  padding: 24px;
  overflow: hidden;
}

.error-banner {
  position: absolute;
  bottom: 16px;
  left: 88px;
  right: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  background: color-mix(in srgb, var(--danger) 15%, var(--panel));
  border: 1px solid var(--danger);
  border-radius: 8px;
  padding: 10px 14px;
  font-size: 12px;
  z-index: 10;
  word-break: break-word;
}

.dismiss {
  margin-left: auto;
  padding: 2px 8px;
  background: transparent;
  border: none;
  color: var(--muted);
}
</style>
