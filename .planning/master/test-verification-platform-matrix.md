# Test & Verification Platform Matrix

## PR gate

| Layer | Required | Command owner | Evidence |
|---|---|---|---|
| FAST | format, lint, typecheck, unit, architecture/schema | versioned local scripts | command result + SHA |
| CORE | Rust/frontend suites, contracts, dependency/security scan, CodeQL | existing workflows + PR-350 | JUnit/TAP/artifact where supported |
| INTEGRATION | real SQLite/migrations, runtime/provider/event/tool boundaries | PR-351/352 | DB/event/tool artifact |
| BROWSER E2E | Playwright goldens | existing `Frontend E2E Chromium`, PR-356 | trace/video/screenshot/report |
| DESKTOP E2E | only when driver evidence exists | PR-358/359 | native driver/OS artifact |

## Nightly

| Layer | Trigger | Policy |
|---|---|---|
| fuzz/property deep | nightly | minimized crash/seed artifact; no external effect |
| load/soak | nightly | bounded resources and leak report |
| chaos | nightly | isolated fault injection and recovery proof |
| protected integration | nightly/manual protected environment | NO_PROOF when credentials unavailable |
| real model eval | nightly/manual | structured assertions, cost limits, no LLM-only verdict |

## Release

| Layer | Required evidence |
|---|---|
| full relevant matrix | exact release SHA/tree and stable check contexts |
| package | checksums, SBOM, provenance, signature when supported |
| install | fresh runner install/launch/smoke artifact per claimed OS |
| upgrade/rollback | real migration/updater evidence when feature exists; otherwise NO_PROOF |
| rehearsal | immutable release report linking requirements to tests/artifacts |

## Cross-platform matrix

| Capability | Linux | Windows | macOS |
|---|---|---|---|
| Rust core | existing CI | existing CI | NO_PROOF until workflow exists |
| Tauri build | existing CI | packaging path exists | NO_PROOF until workflow exists |
| Browser E2E | Chromium existing | future observed runner | future observed runner |
| Native desktop E2E | PR-358/359 | PR-360 | PR-360 |
| Install smoke | PR-370 | PR-370 | future claim only after runner evidence |

No matrix cell is promoted from another OS. Absence is `NO_PROOF`, not PASS.
