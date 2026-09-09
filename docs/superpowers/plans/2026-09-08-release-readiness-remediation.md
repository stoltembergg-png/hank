# Hank Desktop Release Readiness Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the remaining provider, updater, rollback, and platform `PASS limitado`/`NO_PROOF` results into independently verified release evidence without treating fixtures or contract tests as production readiness.

**Architecture:** Keep provider-neutral contracts in `provider-core`, put concrete HTTP/OAuth and OS-secret wiring behind desktop/runtime adapters, and keep the frontend on typed Tauri commands. Persist only account metadata in SQLite; store token material exclusively through `SecureSecretBackend`. The updater must verify an immutable signed manifest before staging, activate atomically, and retain a last-known-good slot for rollback.

**Tech Stack:** Rust 2021, Tokio, SQLx/SQLite migrations, Tauri 2, React/TypeScript/Vite, provider adapter crates, Windows Credential Manager, Linux/macOS native keychains, Node.js contract tests, GitHub Actions.

**Spec:** `.planning/master/releases.md`, `.planning/master/test-verification-platform-matrix.md`, `.planning/master/agent-development-policy.md`, `ARCHITECTURE.md`, and `.planning/contracts/architecture-graph.json`.

## Global Constraints

- A fixture, contract-only run, timeout, `pending`, `partial`, `blocked`, or missing native execution is never reported as `PASS`.
- Every evidence artifact records the exact commit SHA, tree SHA, policy revision, schema version, environment, and artifact digest.
- Secrets never enter SQLite, logs, traces, fixtures, frontend payloads, or release artifacts; only opaque credential references and redacted metadata cross those boundaries.
- `agent-core` remains provider-neutral; concrete providers, Tauri, Tokio, SQLx, network, and OS keychains stay outside it.
- Release publication requires protected signing, checksum/SBOM verification, and successful clean-room install evidence for each claimed OS.
- Each implementation step follows RED → GREEN → REFACTOR and ends with a focused test command and a small commit.

---

### Task 1: Durable credential service and account metadata

**Files:**
- Create: `migrations/0022_provider_accounts.sql`
- Create: `apps/desktop/src-tauri/src/provider_credential_store.rs`
- Modify: `apps/desktop/src-tauri/src/provider_settings.rs`
- Modify: `apps/desktop/src-tauri/src/platform_store.rs`
- Modify: `apps/desktop/src-tauri/src/main.rs`
- Test: `apps/desktop/src-tauri/src/provider_credential_store.rs` and `apps/desktop/src-tauri/src/provider_settings.rs`

**Interfaces:**
- Consumes `provider_core::credentials::{CredentialAccount, CredentialService, CredentialRef}` and `secrets_core::{SecureSecretBackend, SecureSecretStore}`.
- Produces `ProviderCredentialStore<B>` implementing `CredentialService`, plus a SQLite-backed account metadata repository that never stores secret material.

- [ ] **Step 1: Write the failing tests**

Add a migration assertion that `provider_accounts` has a project-scoped primary key and cascades on project deletion. Add a store test that connects an account, drops the store, recreates it against the same SQLite pool, and observes the account as `Unavailable` when the OS backend cannot resolve its secret. Add a Windows-only test that connects synthetic material through `PlatformSecretBackend`, resolves it after recreation, and deletes both the secret and metadata on disconnect.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run `CARGO_BUILD_JOBS=1 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml provider_credential_store --offline --locked`.

Expected: compilation/test failure because the migration, store type, and persistence behavior do not yet exist.

- [ ] **Step 3: Implement the minimal secure store**

Create the table with `project_id`, `provider_id`, `account_id`, `display_name`, `state`, `credential_ref`, and `updated_at`; add a foreign key to `projects(id)` with `ON DELETE CASCADE` and a check constraint for the five allowed states. Implement `ProviderCredentialStore<B>` with an in-memory cache only for bounded status reads, `SecureSecretStore<B>` for all material operations, and SQL upserts for metadata. On restart, report `Unavailable` until the backend successfully resolves the opaque reference; never infer `Connected` from SQLite alone. Wire one `Arc<dyn CredentialService>` into provider settings and chat so disconnect revokes future invocations.

- [ ] **Step 4: Run tests and security gates**

Run `CARGO_BUILD_JOBS=1 cargo fmt --all -- --check`, `CARGO_BUILD_JOBS=1 cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --offline --locked -- -D warnings`, and `CARGO_BUILD_JOBS=1 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --offline --locked`.

Expected: all existing desktop tests plus the restart, cascade, and Windows Credential Manager tests pass; no secret value appears in serialized rows or error messages.

- [ ] **Step 5: Commit**

Commit with `feat(desktop): persist provider account metadata securely`.

### Task 2: Real provider transport and runtime registration

**Files:**
- Create: `apps/desktop/src-tauri/src/provider_transport.rs`
- Create: `apps/desktop/src-tauri/src/provider_runtime.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/chat.rs`
- Modify: `crates/provider-core/src/request.rs`
- Modify: `crates/provider-adapters/openai/src/lib.rs`
- Test: `apps/desktop/src-tauri/src/provider_runtime.rs`, `crates/provider-adapters/openai/tests/provider_contract.rs`

**Interfaces:**
- Consumes `HttpTransport`, `EndpointPolicy`, the secure credential resolver from Task 1, and `ProviderApplicationService`.
- Produces a desktop-owned `ModelProvider` façade for each enabled concrete adapter; the façade receives the validated project/agent/session envelope and resolves credentials only at send time.

- [ ] **Step 1: Write the failing tests**

Add a transport test using a local deterministic server that asserts the request contains no secret literal, uses the configured HTTPS endpoint, honors cancellation/timeout, and maps 401/429/5xx into the existing normalized provider errors. Add a registry test that an enabled OpenAI adapter is discoverable by `ProviderApplicationService` and that an account with no resolvable credential is rejected before network I/O.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run `CARGO_BUILD_JOBS=1 cargo test -p hank-desktop provider_runtime --offline --locked` and `CARGO_BUILD_JOBS=1 cargo test -p provider-adapter-openai --offline --locked`.

Expected: failure because the desktop transport and `ModelProvider` façade are not registered and the adapter currently exposes only synchronous normalized-request helpers.

- [ ] **Step 3: Implement the façade and transport**

Use the existing adapter request/response mapping; add a bounded `reqwest` transport in the desktop shell with HTTPS-only endpoint validation, explicit timeout, cancellation checks, response-size limits, and redacted diagnostics. Extend the runtime request envelope only with validated identity fields required to build `NormalizedRequest`; do not synthesize project or account identity. Register OpenAI first behind an explicit provider configuration and feature gate; leave other descriptors `Unavailable` until their façades have the same tests.

- [ ] **Step 4: Run provider and frontend gates**

Run `CARGO_BUILD_JOBS=1 cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --offline --locked -- -D warnings`, `CARGO_BUILD_JOBS=1 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --offline --locked`, `cd frontend; node node_modules/eslint/bin/eslint.js src`, and `cd frontend; node node_modules/typescript/bin/tsc --noEmit -p tsconfig.json`.

Expected: the mock fixture remains deterministic, OpenAI is registered only with a resolvable secure credential, and unsupported providers remain explicitly unavailable.

- [ ] **Step 5: Commit**

Commit with `feat(desktop): register secure real provider runtime`.

### Task 3: OAuth authorization and provider settings E2E

**Files:**
- Modify: `crates/auth-core/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/provider_settings.rs`
- Modify: `frontend/src/api/provider-settings.ts`
- Modify: `frontend/src/providers/settings/ProviderSettingsPage.tsx`
- Modify: `desktop-e2e/specs/project-lifecycle.e2e.mjs`
- Test: `crates/auth-core/tests/oauth_contract.rs`, `apps/desktop/src-tauri/src/provider_settings.rs`, `frontend/tests/provider_settings_ac_tests.test.tsx`

**Interfaces:**
- Consumes the secure credential store and provider configuration from Tasks 1–2.
- Produces an authorization URL, validated callback listener, connected/revoked account status, and a browser-visible error for invalid, expired, replayed, or cross-project callbacks.

- [ ] **Step 1: Write the failing tests**

Add an OAuth contract asserting that start returns a provider-owned authorization URL containing state and PKCE challenge without exposing the verifier; a valid callback stores the exchanged material in the OS backend; a second callback is replay-rejected; restart reports the account according to backend availability. Extend the desktop E2E to connect, reload, disconnect, and assert that a subsequent chat invocation is denied until reconnection.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run `CARGO_BUILD_JOBS=1 cargo test -p auth-core oauth --offline --locked`, `CARGO_BUILD_JOBS=1 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml provider_settings --offline --locked`, and `cd frontend; npm test -- --run tests/provider_settings_ac_tests.test.tsx`.

Expected: failure because start does not return an authorization URL, the fixture exchange returns only an opaque reference, and the E2E currently covers only a negative callback.

- [ ] **Step 3: Implement the callback path**

Return a redacted authorization URL from the typed command, keep verifier/state server-side, validate provider/account/project/redirect/PKCE before exchange, write material through `SecureSecretStore`, and make disconnect delete the OS secret before marking metadata revoked. Keep fixture behavior behind `HANK_E2E_MOCK_PROVIDER=1`; a release build without configured provider credentials remains `Unavailable`.

- [ ] **Step 4: Run exact-SHA E2E**

Build the release binary with bound `HANK_BUILD_COMMIT_SHA` and `HANK_BUILD_TREE_SHA`, install it into a clean temporary directory, run `desktop-e2e/run-windows.ps1` with the fixture enabled, capture `artifact-identity.json`, screenshots, and logs, then uninstall and verify the executable and uninstaller are gone.

Expected: valid fixture connect/restart/disconnect flow passes; no callback verifier, token, or secret is present in artifacts.

- [ ] **Step 5: Commit**

Commit with `feat(desktop): complete provider OAuth lifecycle`.

### Task 4: Signed updater activation and rollback

**Files:**
- Create: `apps/desktop/src-tauri/src/updater.rs`
- Modify: `apps/desktop/src-tauri/src/main.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `tools/updater-contract.mjs`
- Modify: `tools/updater-contract.spec.mjs`
- Modify: `.github/workflows/ci-auto-updater.yml`
- Test: `apps/desktop/src-tauri/src/updater.rs`, `desktop-e2e/specs/updater-rollback.e2e.mjs`

**Interfaces:**
- Consumes the release manifest/signing envelope and immutable artifact identity.
- Produces typed `check_update`, `stage_update`, `activate_update`, and `rollback_update` commands with atomic activation and a last-known-good marker.

- [ ] **Step 1: Write the failing tests**

Add tests for wrong signature, wrong repository/ref/SHA/tree, digest mismatch, downgrade, interrupted activation, and rollback after a failed health check. Add a desktop E2E that installs version A, stages signed version B, interrupts activation, proves A remains runnable, then activates B and rolls back to A after a synthetic startup failure.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run `node --test tools/updater-contract.spec.mjs` and `CARGO_BUILD_JOBS=1 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml updater --offline --locked`.

Expected: the Node contract tests remain the only passing layer; native commands and interrupted activation tests fail because no live updater exists.

- [ ] **Step 3: Implement fail-closed staging and atomic activation**

Verify the signed manifest and artifact before writing to a bounded staging directory, fsync the staged artifact and marker, swap an atomic pointer, record the previous pointer as last-known-good, and roll back automatically if startup health is not reported before the bounded deadline. Do not permit unattended network endpoints or unsigned artifacts.

- [ ] **Step 4: Run local and CI gates**

Run the updater contract suite, desktop unit tests, and a Windows clean-room install/upgrade/rollback E2E. The GitHub job must publish evidence only after exact SHA/tree verification and must leave `NO_PROOF` when the native job did not run.

- [ ] **Step 5: Commit**

Commit with `feat(desktop): add signed updater rollback path`.

### Task 5: Cross-platform install, upgrade, and stable publication evidence

**Files:**
- Modify: `.github/workflows/desktop-e2e.yml`
- Modify: `.github/workflows/release-prerelease.yml`
- Modify: `.github/workflows/release-milestone.yml`
- Modify: `desktop-e2e/run-linux.sh`
- Create: `desktop-e2e/run-macos.sh`
- Modify: `desktop-e2e/install-smoke-linux.sh`
- Create: `desktop-e2e/install-smoke-macos.sh`
- Modify: `tools/evidence-scope-contract.mjs`
- Test: `tools/release-milestone.spec.mjs`, platform workflow checks

**Interfaces:**
- Consumes the signed artifacts and updater from Tasks 1–4.
- Produces per-OS clean-room install, upgrade, rollback, uninstall, and stable-publication evidence bound to one immutable SHA/tree.

- [ ] **Step 1: Write the failing evidence checks**

Add workflow assertions that a stable publication is impossible when any claimed OS lacks a native install report, when upgrade/rollback is `NO_PROOF`, or when the signed manifest identity differs from the built tree. Add macOS runner steps for the same artifact identity and lifecycle assertions already used on Windows/Linux.

- [ ] **Step 2: Run the checks to verify the current state is incomplete**

Run `node --test tools/release-milestone.spec.mjs tools/evidence-scope-contract.spec.mjs` and inspect the workflow artifacts. Expected: contract checks pass while native Linux/macOS and stable protected publication remain `NO_PROOF` on this workstation.

- [ ] **Step 3: Implement per-OS runners and evidence binding**

Keep OS-specific commands in their native jobs, upload JSON reports with SHA/tree/version/digest, and make the promotion job require every report plus signature, checksum, SBOM, and provenance verification. Never reuse Windows evidence for Linux/macOS.

- [ ] **Step 4: Run protected CI and reconcile the exact SHA**

Push the release branch only after explicit approval, wait for required checks on that exact SHA, execute Windows/Linux/macOS install and rollback jobs, then rerun the stable promotion verification after any rebase or delayed check.

- [ ] **Step 5: Commit**

Commit with `ci(release): require native platform and rollback evidence`.

## Completion Criteria

- Provider settings and chat use the same secure, project-scoped credential service; valid OAuth connect/restart/disconnect and real provider invocation are proven with redacted artifacts.
- Updater staging, activation, interruption recovery, and rollback have native E2E evidence; contract-only tests are not promoted to PASS.
- Windows, Linux, and macOS clean-room install evidence exists for the exact release SHA/tree, including upgrade and rollback.
- Protected signing, SBOM, checksums, provenance, and stable publication verification are green on the same SHA/tree.
- Any remaining missing external runner or protected-CI evidence is explicitly `NO_PROOF`, preventing release promotion.
