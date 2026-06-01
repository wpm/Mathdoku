import { defineConfig } from '@playwright/test';
import path from 'path';

// Isolated config for the WASM-everything (`--features web`) preview build,
// where the editor runs entirely in-process against mathdoku-designer-core with
// no Tauri backend and no window.__TAURI__ stubs (issue #74). The default
// playwright.config.ts (testDir ./tests) keeps driving the Tauri-path build
// against :1420; this config serves the web build on its own port and dist dir
// so the two never collide. Run with `npm run test:web`.
const chromiumExecutable = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;

export default defineConfig({
  testDir: './tests-web',
  timeout: 30000,
  use: {
    baseURL: 'http://localhost:1421',
    browserName: 'chromium',
    trace: 'retain-on-failure',
    launchOptions: {
      args: ['--no-sandbox'],
      ...(chromiumExecutable ? { executablePath: chromiumExecutable } : {}),
    },
  },
  // Auto-start a web-feature trunk build on a dedicated port/dist so it can run
  // alongside the Tauri-path server. The cold wasm build is ~1 min.
  webServer: {
    command: 'trunk serve --features web --dist dist-web --port 1421',
    cwd: path.resolve(__dirname, '..'),
    url: 'http://localhost:1421',
    reuseExistingServer: true,
    timeout: 180_000,
  },
});
