import { reactive } from 'vue'
import type { JobProgress, ModelInfo, Settings } from './types'

export type Route = 'single' | 'batch' | 'settings'

export interface BatchFile {
  path: string
  name: string
  status: 'pending' | 'done' | 'skipped' | 'failed'
  message?: string
}

interface AppState {
  route: Route
  dragActive: boolean
  version: string
  error: string | null
  settings: Settings | null
  job: {
    running: boolean
    progress: JobProgress | null
  }
  single: {
    input: string | null
    output: string | null
  }
  batch: {
    files: BatchFile[]
    outDir: string | null
  }
  models: ModelInfo[]
  download: {
    key: string | null
    downloadedBytes: number
    totalBytes: number | null
  }
}

const state = reactive<AppState>({
  route: 'single',
  dragActive: false,
  version: '',
  error: null,
  settings: null,
  job: { running: false, progress: null },
  single: { input: null, output: null },
  batch: { files: [], outDir: null },
  models: [],
  download: { key: null, downloadedBytes: 0, totalBytes: null },
})

export function useApp(): AppState {
  return state
}

export function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  }
  if (bytes >= 1024 * 1024) {
    return `${Math.round(bytes / (1024 * 1024))} MB`
  }
  return `${Math.round(bytes / 1024)} KB`
}
