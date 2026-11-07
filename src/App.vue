<script setup lang="ts">
import { onMounted } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { listen } from '@tauri-apps/api/event'
import Sidebar from './components/Sidebar.vue'
import SingleView from './views/SingleView.vue'
import BatchView from './views/BatchView.vue'
import SettingsView from './views/SettingsView.vue'
import { useApp, startBatch, type BatchFile } from './store'
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
    // bust the webview cache — overwritten outputs keep their path/URL
    store.single.version = Date.now()
    if (store.route === 'single') {
      const written = done.outputs[0]?.output
      const skippedOut = done.skipped[0]?.output
      if (written) {
        store.single.output = written
        store.single.skipped = false
      } else if (skippedOut) {
        // result file already existed — show it, flagged as not freshly processed
        store.single.output = skippedOut
        store.single.skipped = true
      } else {
        // nothing produced: back to the drop state, error banner carries the reason
        store.single.input = null
        const failure = done.failed[0]
        if (failure) store.error = failure.message
      }
    } else {
      // one pass per outcome: rows created from progress events are updated,
      // files that never got an event (skipped up-front, or missed while on
      // another view) are backfilled
      const outcomes: { input: string; status: BatchFile['status']; message?: string }[] = [
        ...done.outputs.map((o) => ({ input: o.input, status: 'done' as const })),
        ...done.skipped.map((s) => ({
          input: s.input,
          status: 'skipped' as const,
          message: 'result already exists',
        })),
        ...done.failed.map((f) => ({ input: f.input, status: 'failed' as const, message: f.message })),
      ]
      for (const o of outcomes) {
        const row = store.batch.files.find((r) => r.path === o.input)
        if (row) {
          row.status = o.status
          row.message = o.message
        } else {
          store.batch.files.push({
            path: o.input,
            name: fileName(o.input),
            status: o.status,
            message: o.message,
          })
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
    store.downloads[p.key] = { downloadedBytes: p.downloadedBytes, totalBytes: p.totalBytes }
    if (p.done) {
      delete store.downloads[p.key]
      store.models = await api.listModels().catch(() => store.models)
    }
  })

  await listen<{ key: string; message: string }>('noground://model-error', async (e) => {
    if (e.payload.message !== 'cancelled') {
      store.error = e.payload.message
    }
    delete store.downloads[e.payload.key]
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
    startBatch()
  } else {
    store.route = 'single'
    store.single.input = entries[0]?.path ?? null
    store.single.output = null
    store.single.skipped = false
    store.single.model = store.settings?.model ?? null
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
