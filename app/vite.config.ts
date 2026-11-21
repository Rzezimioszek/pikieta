import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

export default defineConfig({
  // Ścieżki względne: ta sama paczka działa pod GitHub Pages i w Tauri.
  base: './',
  plugins: [svelte()],
  // Wątek roboczy GML importuje moduł WASM, więc musi być modułem ES.
  worker: { format: 'es' },
  server: { port: 5173, strictPort: true },
})
