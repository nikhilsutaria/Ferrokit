import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    // `npm run dev` talks to a Ferrokit server on :8080 for /api.
    proxy: { '/api': 'http://localhost:8080' },
  },
  build: { target: 'es2022', chunkSizeWarningLimit: 1500 },
  worker: { format: 'es' },
  test: { include: ['src/**/*.test.ts'] },
});
