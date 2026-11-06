import { invoke } from '@tauri-apps/api/core'
import type { ModelInfo, ModelState, PathEntry, Settings } from './types'

export const api = {
  getSettings: () => invoke<Settings>('get_settings'),
  setSettings: (settings: Settings) => invoke<Settings>('set_settings', { settings }),
  inspectPaths: (paths: string[]) => invoke<PathEntry[]>('inspect_paths', { paths }),
  processPaths: (paths: string[]) => invoke<void>('process_paths', { paths }),
  cancelJob: () => invoke<boolean>('cancel_job'),
  listModels: () => invoke<ModelInfo[]>('list_models'),
  ensureModel: (key: string) => invoke<ModelState>('ensure_model', { key }),
  cancelModelDownload: (key: string) => invoke<boolean>('cancel_model_download', { key }),
}
