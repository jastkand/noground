<script setup lang="ts">
import { computed } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { convertFileSrc } from '@tauri-apps/api/core'
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

const stageLabel = computed(() => {
  const stage = store.job.progress?.stage
  if (!stage || stage === 'decode') return 'Reading image…'
  if (stage === 'model') return 'Loading model…'
  if (stage === 'infer') return 'Removing background…'
  return 'Saving…'
})

const previewSrc = computed(() =>
  store.single.input ? convertFileSrc(store.single.input) : null,
)

const modelChanged = computed(
  () => !!store.single.model && !!store.settings?.model && store.settings.model !== store.single.model,
)

const modelLabel = computed(() => {
  const key = store.single.model
  if (!key) return ''
  return store.models.find((m) => m.key === key)?.label ?? key
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
  } else if (paths.length === 1) {
    store.single.input = paths[0]
    store.single.output = null
    store.single.skipped = false
    store.single.model = store.settings?.model ?? null
    store.job.running = true
  }
  try {
    await api.processPaths(paths)
  } catch (e) {
    store.job.running = false
    store.single.input = null
    store.error = String(e)
  }
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
  store.single.skipped = false
  store.single.model = null
}

// re-run the current image with the current settings (new model, etc.);
// one-shot overwrite so the previous result gets replaced
async function regenerate() {
  if (!store.single.input) return
  store.single.output = null
  store.single.skipped = false
  store.single.model = store.settings?.model ?? null
  store.job.running = true
  try {
    await api.processPaths([store.single.input], true)
  } catch (e) {
    store.job.running = false
    store.error = String(e)
  }
}
</script>

<template>
  <div class="view">
    <header>
      <h1>Remove background</h1>
      <p class="muted">Runs fully offline — nothing leaves this Mac.</p>
    </header>

    <DropZone v-if="!store.single.input && !store.job.running" />

    <div v-if="store.job.running && previewSrc" class="preview">
      <img class="preview-img" :src="previewSrc" alt="processing" draggable="false" />
      <div class="overlay">
        <div class="spinner"></div>
        <span>{{ stageLabel }}</span>
        <button class="compact" @click="cancel">Cancel</button>
      </div>
    </div>

    <div v-else-if="store.job.running" class="panel running">
      <ProgressBar :value="progressPct" label="Preparing…" />
      <button class="compact" @click="cancel">Cancel</button>
    </div>

    <template v-else-if="store.single.output">
      <p v-if="store.single.skipped" class="notice panel">
        Result file already existed — showing it below, nothing was reprocessed.
      </p>
      <CompareSlider :before="store.single.input!" :after="store.single.output" />
      <div class="actions">
        <button @click="revealItemInDir(store.single.output!)">Show in Finder</button>
        <button :class="{ primary: modelChanged || store.single.skipped }" @click="regenerate">
          Re-generate
        </button>
        <button @click="startOver">Start over</button>
      </div>
      <p class="path muted">
        {{ store.single.output }}
        <span v-if="store.single.model">
          · model: {{ modelLabel }}
          <template v-if="modelChanged"> — changed in Settings, Re-generate to apply</template>
        </span>
      </p>
    </template>

    <div v-if="!store.single.input && !store.job.running" class="actions">
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

.preview {
  position: relative;
  display: flex;
  justify-content: center;
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg-deep);
}

.preview-img {
  display: block;
  max-width: 100%;
  max-height: calc(100vh - 220px);
  object-fit: contain;
  user-select: none;
}

.overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  background: color-mix(in srgb, var(--bg-deep) 55%, transparent);
  font-size: 13px;
  color: var(--text);
}

.spinner {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  border: 4px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-top-color: var(--accent);
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.notice {
  margin: 0;
  padding: 10px 14px;
  font-size: 12px;
  color: var(--muted);
  border-radius: 8px;
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
