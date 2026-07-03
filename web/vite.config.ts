import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  plugins: [svelte()],
  server: {
    proxy: {
      // Game WebSocket → golf-server during development.
      '/ws': {
        target: 'ws://localhost:8080',
        ws: true,
      },
    },
  },
})
