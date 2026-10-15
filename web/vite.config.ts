import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

const BACKEND = process.env.CADENCE_BACKEND ?? 'http://127.0.0.1:8787'

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: BACKEND, changeOrigin: true },
      '/files': { target: BACKEND, changeOrigin: true },
      '/ws': { target: BACKEND, ws: true, changeOrigin: true },
    },
  },
})
