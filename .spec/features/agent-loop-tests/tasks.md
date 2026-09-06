# Tasks: agent-loop-tests

> feature: agent-loop-tests
> card: PR-264

## T-2500 — Implement bounded loop contract [concluida]

- Refs: US-2500, AC-2501, AC-2502, AC-2503, AC-2504, AC-2505
- Arquivos: `crates/test-support/src/agent_loop.rs`, `crates/test-support/tests/agent_loop_contract.rs`, `crates/test-support/src/lib.rs`
- Evidência: contrato Rust 5/5, fmt, check e clippy PASS.

## T-2501 — Add executable evidence runner [concluida]

- Refs: US-2500, AC-2501, AC-2502, AC-2503, AC-2504, AC-2505
- Arquivos: `.github/workflows/ci-agent-loop.yml`, `.github/workflows/onp-sdd-evidence.yml`, `docs/agent-loop-tests.md`, `onpspec.config.json`, `tools/security/agent-loop-feature-tests.mjs`
- Evidência: runner TAP 5/5 e verify agent-loop-tests 5/5.
