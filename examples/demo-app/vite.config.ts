import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasmPack from 'vite-plugin-wasm-pack';
import inject from '@rollup/plugin-inject'

export default defineConfig({
  plugins: [
    react(),
    wasmPack([], ['race-sdk'])],
  preview: {
    open: true
  },
  resolve: {
    alias: {
      buffer: "buffer/"
    }
  },
  build: {
    rollupOptions: {
      plugins: [inject({ Buffer: ['buffer', 'Buffer'] })],
    },
  },
})
