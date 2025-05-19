import { defineConfig } from 'vite';
import path from 'path';

// https://vitejs.dev/config
export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  },
  build: {
    outDir: '.vite/build',
    lib: {
      entry: path.resolve(__dirname, 'src/main.ts'),
      formats: ['cjs'],
      fileName: () => 'main.js',
    },
    rollupOptions: {
      external: [
        'electron',
        'electron-squirrel-startup',
        'path',
        'fs',
        'os',
        'child_process',
        'crypto',
        'yaml'
      ],
      output: {
        format: 'cjs',
      }
    },
    sourcemap: true,
    minify: false,
  }
});