import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { fileURLToPath } from 'node:url'

export default defineConfig({
  plugins: [react()],
  base: './',
  build: {
    outDir: fileURLToPath(new URL('../src/easy_personal/dashboard/static', import.meta.url)),
    emptyOutDir: true,
    sourcemap: false,
    modulePreload: { polyfill: false },
  },
})
