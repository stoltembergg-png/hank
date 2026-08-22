# Desktop development and E2E

## Launching the desktop app

Do not start the desktop shell with `cargo run` from `apps/desktop/src-tauri`. That bypasses Tauri CLI lifecycle hooks, including `beforeDevCommand`, so the Vite server for `devUrl` may not be running.

Use the repository launcher instead:

```bash
node tools/run-desktop-dev.mjs
```

It invokes the frontend-owned Tauri CLI from the Tauri crate directory. Tauri then starts the configured Vite server and loads `http://localhost:1420` in development.

## Packaged frontend assets

The desktop bundle must load only relative assets from `frontend/dist`. Absolute `/assets/...` references are invalid for packaged local resources and can result in file-not-found failures.

Validate a production artifact with:

```bash
npm --prefix frontend run build
node tools/verify-desktop-frontend-assets.mjs frontend/dist
```

## Browser E2E

The browser E2E suite runs the Vite frontend in Chromium and verifies that the project workspace renders and the create-project form opens:

```bash
npm --prefix frontend run test:e2e
```

The CI workflow `.github/workflows/frontend-e2e.yml` installs Chromium, runs the suite, and uploads Playwright traces, screenshots, videos, and HTML reports on failure.

This browser suite verifies the application surface and asset path behavior. A future Tauri WebDriver suite is still needed for native window lifecycle, IPC and OS-specific desktop behavior; browser E2E does not claim that coverage.
