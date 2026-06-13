import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  testIgnore: '**/_*.spec.ts', // _shots.spec.ts (screenshots) no corre en la suite real
  timeout: 120_000, // AVIF encode in wasm is slow
  use: { baseURL: 'http://localhost:4173' },
  webServer: {
    command: 'npm run preview',
    port: 4173,
    reuseExistingServer: !process.env.CI,
  },
});
