import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Site statique ; un moteur WebAssembly éventuel tourne dans un Web Worker module.
export default defineConfig({
  plugins: [svelte()],
  worker: { format: 'es' },
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
});
