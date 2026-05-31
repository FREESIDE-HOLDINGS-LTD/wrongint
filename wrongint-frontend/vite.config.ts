import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Dev server proxies the API to the backend so the browser talks to a single
// origin (no CORS, same URLs as production).
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:8080',
      '/metrics': 'http://localhost:8080',
    },
  },
})
