export interface Settings {
  model: string
  outputDir: string | null
  overwrite: boolean
  format: 'png' | 'jpeg'
}

export interface PathEntry {
  path: string
  name: string
  isDir: boolean
}

export type JobStage = 'model' | 'decode' | 'infer' | 'encode'

export interface JobProgress {
  stage: JobStage
  index: number
  total: number
  file: string
}

export interface JobOutput {
  input: string
  output: string
}

export interface JobFailure {
  input: string
  message: string
}

export interface JobSkipped {
  input: string
  output: string
}

export interface JobDone {
  outputs: JobOutput[]
  skipped: JobSkipped[]
  failed: JobFailure[]
  cancelled: boolean
  outDir: string | null
}

export interface ModelInfo {
  key: string
  label: string
  approxBytes: number
  installed: boolean
  diskBytes: number | null
}

export type ModelState = { status: 'ready'; path: string } | { status: 'in-progress' }

export interface ModelDownloadProgress {
  key: string
  downloadedBytes: number
  totalBytes: number | null
  done: boolean
}

export interface CliStatus {
  installed: boolean
  linkPath: string
  target: string | null
  binDirOnPath: boolean
  devBuild: boolean
}
