<script setup lang="ts">
import { onMounted } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useApp, formatBytes } from '../store'
import { api } from '../api'

const store = useApp()

onMounted(async () => {
  store.models = await api.listModels()
})

async function save() {
  if (store.settings) {
    store.settings = await api.setSettings(store.settings)
  }
}

async function pickOutputDir() {
  const picked = await open({ directory: true })
  if (typeof picked === 'string' && store.settings) {
    store.settings.outputDir = picked
    await save()
  }
}

async function resetOutputDir() {
  if (store.settings) {
    store.settings.outputDir = null
    await save()
  }
}

async function selectModel(key: string) {
  if (store.settings) {
    store.settings.model = key
    await save()
  }
}

async function downloadModel(key: string) {
  store.download.key = key
  store.download.downloadedBytes = 0
  store.download.totalBytes = null
  await api.ensureModel(key)
}

async function cancelDownload(key: string) {
  await api.cancelModelDownload(key)
}

function isDownloading(key: string): boolean {
  return store.download.key === key && !downloadDone(key)
}

function downloadDone(key: string): boolean {
  return (
    store.download.key === key &&
    store.download.totalBytes !== null &&
    store.download.downloadedBytes >= store.download.totalBytes
  )
}
</script>

<template>
  <div class="view">
    <header>
      <h1>Settings</h1>
    </header>

    <section v-if="store.settings" class="panel settings">
      <div class="row">
        <div class="label">
          Model
          <p class="muted">Larger models give cleaner edges; downloads are cached locally.</p>
        </div>
      </div>
      <div v-for="model in store.models" :key="model.key" class="row model-row">
        <div class="model-info">
          <label class="model-name">
            <input
              type="radio"
              name="model"
              :value="model.key"
              :checked="store.settings.model === model.key"
              :disabled="!model.installed"
              @change="selectModel(model.key)"
            />
            {{ model.label }}
            <span v-if="store.settings.model === model.key" class="accent">active</span>
          </label>
        </div>
        <div class="control">
          <span v-if="model.installed" class="muted">installed</span>
          <template v-else-if="isDownloading(model.key)">
            <span class="muted">
              {{ formatBytes(store.download.downloadedBytes) }}
              <template v-if="store.download.totalBytes">
                / {{ formatBytes(store.download.totalBytes) }}
              </template>
            </span>
            <button class="danger" @click="cancelDownload(model.key)">Cancel</button>
          </template>
          <button v-else @click="downloadModel(model.key)">
            Download (~{{ formatBytes(model.approxBytes) }})
          </button>
        </div>
      </div>
    </section>

    <section v-if="store.settings" class="panel settings">
      <div class="row">
        <div class="label">
          Output folder
          <p class="muted">Where results are saved. Default: next to each source image.</p>
        </div>
        <div class="control">
          <button @click="pickOutputDir">Choose…</button>
          <button v-if="store.settings.outputDir" class="danger" @click="resetOutputDir">
            Reset
          </button>
        </div>
      </div>
      <p v-if="store.settings.outputDir" class="path muted">{{ store.settings.outputDir }}</p>

      <div class="row">
        <div class="label">
          Overwrite existing
          <p class="muted">Re-process images whose results already exist.</p>
        </div>
        <div class="control">
          <input v-model="store.settings.overwrite" type="checkbox" @change="save" />
        </div>
      </div>

      <div class="row">
        <div class="label">
          Output format
          <p class="muted">JPEG flattens transparency onto white.</p>
        </div>
        <div class="control">
          <select v-model="store.settings.format" @change="save">
            <option value="png">PNG</option>
            <option value="jpeg">JPEG</option>
          </select>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.view {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
  max-width: 720px;
  overflow-y: auto;
}

header h1 {
  margin: 0 0 2px;
  font-size: 20px;
}

.settings {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 16px;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 0;
}

.row + .row {
  border-top: 1px solid var(--border);
}

.label {
  font-weight: 600;
}

.label p {
  margin: 2px 0 0;
  font-weight: 400;
  font-size: 12px;
}

.control {
  display: flex;
  gap: 8px;
  align-items: center;
}

.model-row {
  font-weight: 400;
}

.model-name {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.accent {
  color: var(--accent);
  font-size: 11px;
  font-weight: 600;
}

.path {
  margin: -6px 0 8px;
  font-size: 11px;
  word-break: break-all;
  user-select: text;
}
</style>
