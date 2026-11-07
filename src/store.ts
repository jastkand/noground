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
    skipped: boolean
    model: string | null
    version: number
  }
  batch: {
    files: BatchFile[]
    outDir: string | null
  }
  models: ModelInfo[]
  downloads: Record<string, { downloadedBytes: number; totalBytes: number | null }>
}

const state = reactive<AppState>({
  route: 'single',
  dragActive: false,
  version: '',
  error: null,
  settings: null,
  job: { running: false, progress: null },
  single: { input: null, output: null, skipped: false, model: null, version: 0 },
  batch: { files: [], outDir: null },
  models: [],
  downloads: {},
})

export function useApp(): AppState {
  return state
}

/// switch to the batch view for a fresh multi-file run
export function startBatch() {
  state.route = 'batch'
  state.batch.files = []
  state.batch.outDir = null
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
