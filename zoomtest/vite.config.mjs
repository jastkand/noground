import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'node:path'

export default defineConfig({
  root: import.meta.dirname,
  publicDir: 'public',
  plugins: [vue()],
  resolve: {
    alias: {
      '@tauri-apps/api/core': resolve(import.meta.dirname, 'tauri-stub.ts'),
    },
  },
  server: { port: 1430, strictPort: true },
})
