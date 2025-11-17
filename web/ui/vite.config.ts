import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { viteSingleFile } from 'vite-plugin-singlefile';

const thisDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(thisDir, '..');

export default defineConfig({
  plugins: [
    react(),
    viteSingleFile({
      removeViteModuleLoader: true,
      useRecommendedBuildConfig: true,
    }),
  ],
  resolve: {
    alias: {
      '@schemaui/types': path.resolve(workspaceRoot, 'types'),
      '@': path.resolve(thisDir, './src'),
    },
  },
  server: {
    fs: {
      allow: [workspaceRoot],
    },
  },
  build: {
    target: 'esnext',
    assetsInlineLimit: 100_000_000,
    cssCodeSplit: false,
    outDir: '../dist',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
      },
    },
  },
});
