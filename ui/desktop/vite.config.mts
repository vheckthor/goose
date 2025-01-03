import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

// https://vitejs.dev/config
export default defineConfig({
  plugins: [react()],
  build: {
    target: 'esnext',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'src/main.ts'),
        index: resolve(__dirname, 'index.html'),
      },
    },
  },
  worker: {
    format: 'es',
    plugins: () => [react()],
    rollupOptions: {
      output: {
        format: 'es',
        chunkFileNames: '[name]-[hash].js',
      }
    }
  },
  optimizeDeps: {
    exclude: ['@huggingface/transformers']
  }
});