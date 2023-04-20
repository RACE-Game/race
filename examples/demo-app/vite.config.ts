import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasmPack from 'vite-plugin-wasm-pack';
// import { NodeGlobalsPolyfillPlugin } from '@esbuild-plugins/node-globals-polyfill'
// import { NodeModulesPolyfillPlugin } from '@esbuild-plugins/node-modules-polyfill'

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
  }
  // define: {
  //   global: {}
  // },
  // optimizeDeps: {
  //   esbuildOptions: {
  //     plugins: [
  //       // NodeGlobalsPolyfillPlugin({ buffer: true }),
  //       NodeModulesPolyfillPlugin(),
  //     ],
  //   }
  // },
})
