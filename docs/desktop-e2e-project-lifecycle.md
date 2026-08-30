# Desktop E2E — Project Lifecycle

## Audit conclusion

`frontend-e2e.yml` runs Playwright against Chromium and does not start Tauri. Existing frontend
coverage includes tests that install `window.__TAURI_INVOKE__`; those tests are intentionally
separate from this gate and do not prove native IPC, Rust commands, SQLite, migrations, or restart
persistence.

The real desktop bridge is registered in `apps/desktop/src-tauri/src/confirmations.rs` and exposes
`create_project`, `list_projects`, `get_project`, `update_project`, and `archive_project`. The
commands use the real Project services and SQLite repository. `build-tauri.yml` currently compiles
and tests the shell but does not open a window or exercise IPC.

## Gate architecture

`desktop-e2e/` uses the W3C WebDriver protocol directly over Node's built-in `fetch`. It does not
use Playwright Chromium, does not inject `window.__TAURI_INVOKE__`, and has no Project mocks. The
real Tauri executable is launched by `tauri-driver`:

- Ubuntu: `WebKitWebDriver` plus `xvfb`;
- Windows: Microsoft Edge WebDriver compatible with the installed WebView2 runtime.

The gate uses an isolated file-backed SQLite directory supplied through `HANK_E2E_APP_DATA_DIR`.
The override is accepted only by debug builds and fails closed in release builds. The same path is
used after each restart and is removed after success; on failure it is uploaded as a diagnostic
artifact.

## Lifecycle asserted

1. empty startup and Projects ready state;
2. create with name, owner, and description;
3. list and open;
4. ID, status, owner, and description verification;
5. create and list a real project/agent-scoped Session through Tauri IPC and SQLite;
6. update and UI confirmation;
7. restart #1 and file-backed persistence verification;
8. archive through the real UI;
9. restart #2 and archived persistence verification;
10. final clean shutdown.

Every failure captures the current phase, screenshot, WebDriver/tauri-driver logs, and app-data
when available. The workflow has an exact aggregator check named `Desktop E2E / Project Lifecycle`
that fails if either Ubuntu or Windows fails.

## Release enforcement

The prerelease workflow waits for the Desktop E2E check on the exact post-merge SHA. Stable
milestone promotion independently verifies a successful `Desktop E2E / Project Lifecycle` check
for the exact prerelease commit before publishing.

Branch protection is intentionally not changed until the new workflow has completed successfully
on the PR head in both supported operating systems. This avoids registering an unproven required
context or creating a permanently blocked main branch.

## Negative mutation evidence

The real mutation test is a release-blocking follow-up validation of this PR: remove one Project
command from `generate_handler!`, rebuild the desktop binary, and run the same WebDriver lifecycle.
The expected result is an IPC/unknown-command failure before create/list succeeds. The mutation is
never committed or pushed. It remains `NO_PROOF` until a runner with native WebKit/Windows
dependencies executes it; local Linux cannot perform it because `javascriptcoregtk-4.1` is absent.
