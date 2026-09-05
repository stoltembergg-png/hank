# Tasks — PR-261 (fuzz-tests)

**ID:** PR-261
**Planned card:** T-2200

## T-2200 — Add bounded fuzz harness

**Owner:** Hermes
**Depends on:** PR-260 (security-tests), PR-232, PR-175, PR-193, PR-255, PR-259

### Sub-tasks

- [x] T-2200.1 — `crates/test-support/src/fuzz.rs` with `FuzzTarget` trait, `FuzzHarness`, `FuzzOutcome`, `FuzzReport`, `FuzzCrash`, `FuzzLimits`, `run_target`, `run_all_targets`, `digest_corpus`, `digest_runner`, `replay_crash`, `verify_no_secrets`.
- [x] T-2200.2 — Register seven FuzzTargets (`FT-001..FT-007`) over `security-core`, `agent-protocol`, `workflow-core`, `recovery-core`, `secrets-core`, `migration-core`, release metadata.
- [x] T-2200.3 — `crates/test-support/tests/fuzz_contract.rs` with 9 tests @spec:AC-2201..AC-2207 (1 per AC + 2 regression).
- [x] T-2200.4 — `crates/test-support/fuzz-corpus/FT-00N/seed-N.bin` with synthetic inputs (no secrets).
- [x] T-2200.5 — `docs/security/fuzz-manifest.json` with `FT-001..FT-007`, schema_version 1, runner_digest placeholder, manifest_revision.
- [x] T-2200.6 — `tools/security/fuzz-runner.mjs` Node runner with TAP output.
- [x] T-2200.7 — `tools/security/security-feature-tests.mjs` extended to chain fuzz after security-tests in TAP.
- [x] T-2200.8 — `.github/workflows/ci-fuzz.yml` with `permissions: contents: read`, `pull_request` + `push: main`, runs smoke.
- [x] T-2200.9 — `onpspec.config.json` extended with `fuzz-tests` command and TAP reporter; `testGlobs` extended with `crates/test-support/tests/fuzz*.rs`.
- [x] T-2200.10 — `.github/workflows/onp-sdd-evidence.yml` extended with `Verify fuzz tests` step.
- [x] T-2200.11 — `docs/fuzz-tests.md` and `tools/security/README.md` updated.

### Evidence (local, 2026-09-05)

- cargo fmt -p test-support -- --check PASS
- cargo clippy -p test-support --all-targets --locked --offline -- -D warnings PASS
- cargo test -p test-support --test fuzz_contract --locked --offline -> 9 passed
- node --test tools/security/fuzz-runner.spec.mjs -> tests pass
- node tools/security/fuzz-runner.mjs --out security/reports/fuzz.json -> pass
- CI=1 node tools/ci/run-onp-spec.mjs verify fuzz-tests -> 7/7 ACs PASS
- tools/commit-message-lint.mjs -> 1/1 PASS
- bash tools/ci/run-actionlint.sh PASS

### Files

- `crates/test-support/src/fuzz.rs` (new)
- `crates/test-support/src/lib.rs` (add `pub mod fuzz;`)
- `crates/test-support/tests/fuzz_contract.rs` (new)
- `crates/test-support/fuzz-corpus/FT-001/seed-001.bin` (new, synthetic)
- `crates/test-support/fuzz-corpus/FT-002/seed-001.bin` (new, synthetic)
- `crates/test-support/fuzz-corpus/FT-003/seed-001.bin` (new, synthetic)
- `crates/test-support/fuzz-corpus/FT-004/seed-001.bin` (new, synthetic)
- `crates/test-support/fuzz-corpus/FT-005/seed-001.bin` (new, synthetic)
- `crates/test-support/fuzz-corpus/FT-006/seed-001.bin` (new, synthetic)
- `crates/test-support/fuzz-corpus/FT-007/seed-001.bin` (new, synthetic)
- `docs/security/fuzz-manifest.json` (new)
- `tools/security/fuzz-runner.mjs` (new)
- `tools/security/fuzz-runner.spec.mjs` (new)
- `tools/security/security-feature-tests.mjs` (extended)
- `.github/workflows/ci-fuzz.yml` (new)
- `onpspec.config.json` (extended)
- `.github/workflows/onp-sdd-evidence.yml` (extended)
- `.spec/features/fuzz-tests/spec.md` (new)
- `.spec/features/fuzz-tests/tasks.md` (new)
- `docs/fuzz-tests.md` (new)
- `tools/security/README.md` (extended)
