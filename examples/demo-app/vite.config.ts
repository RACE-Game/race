import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasmPack from 'vite-plugin-wasm-pack';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react(), wasmPack([], ['race-sdk'])],
  preview: {
    open: true
  },
})
