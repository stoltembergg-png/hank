# Test & Verification Platform — Master Plan

**Status:** `PLANNED / EXECUTION BLOCKED UNTIL PR-345 PASS+MERGED`  
**Authority:** approved SDD → PR-001..PR-270 immutable queue → PR-271..PR-345 Harness queue → architecture/ADRs → repository evidence → this extension.

## Entry gate

No Test & Verification Platform implementation card may start until:

1. PR-345 Harness V2 integration gate is merged with required checks PASS on its exact merge SHA;
2. PR-345 artifacts prove its required cross-component E2Es;
3. PR-346 audits actual coverage against this plan and classifies every test as EXISTS, PARTIAL, MISSING, WEAK or NOT_APPLICABLE;
4. the Test Platform queue validator proves PR-001..PR-345 queue artifacts are unchanged and the new graph is acyclic.

The browser E2E introduced by PR #167 is existing baseline evidence. It does not claim desktop-native, multi-agent, recovery, release or external-integration E2E coverage.

## Current test gap analysis

| Validation class | Current state | Evidence | Planned response |
|---|---|---|---|
| Rust unit/domain/policy | EXISTS | `crates/*/src` unit tests, `cargo test --workspace --locked` | catalog/traceability, property testing for critical invariants |
| Provider/tool contracts | EXISTS/PARTIAL | provider adapter contracts; `tool-core/tests/*_contract.rs` | reusable contract-kit and mandatory adapter matrix |
| Application/SQLite integration | PARTIAL | runtime repository, migration and service contracts | real DB harness, upgrade/corrupt/concurrency/recovery matrix |
| Negative/security tests | PARTIAL | permission/path/symlink/provider/malformed fixtures | unified negative catalog, threat corpus and mutation/property/fuzz pilots |
| Frontend component/accessibility | EXISTS/PARTIAL | Vitest/Testing Library, accessible names | state/error/keyboard/responsive coverage catalog |
| Browser E2E | EXISTS, narrow | Playwright Chromium workflow, PR #167 | Golden web flows and artifact policy |
| Native desktop E2E | MISSING | Tauri tests inspect configuration/source; no driver/window automation | Tauri-driver feasibility then Linux/Windows/macOS native E2E matrix |
| Single/multi-agent/recovery E2E | MISSING until Harness capabilities exist | Harness queue plans them; no runtime evidence yet | deterministic MockProvider/virtual tools/clock then E2Es after PR-345 |
| Fuzz/property/mutation | MISSING | no cargo-fuzz/proptest/mutation dependency detected | targeted pilots after risk/benefit audit, nightly only when costly |
| Performance/load/soak/chaos | MISSING | no benchmark/load/chaos harness | measurement baseline, nightly controlled suites, fault injection |
| CI hardening/static/security | EXISTS/PARTIAL | CodeQL, audit, workflow integrity, pinned actions, `persist-credentials: false`, ONP | formal check registry, security negative corpus, required-context reconciliation |
| Cross-platform build/package | PARTIAL | Rust Windows build, Tauri Linux build, Windows prerelease installer | native behavior and installation matrix; macOS still NO_PROOF |
| Install/upgrade/rollback/release E2E | PARTIAL/WEAK | prerelease manifest/checksum/Windows package; no clean install/upgrade/rollback proof | clean-room installation, upgrade/rollback and release rehearsal gates |
| External integrations | NOT_APPLICABLE without protected credentials | no Telegram/GitHub/provider sandbox E2E proof | protected optional suites producing NO_PROOF when unavailable |

## Test architecture

```text
Requirement / AC
  → Test Catalog entry
  → deterministic fixture + environment class
  → test command / CI gate
  → structured report/artifact
  → exact SHA/tree/policy evidence
```

### Test classes

- **FAST:** format, lint, typecheck, unit, architecture, schema, selected contracts.
- **CORE:** workspace/frontend tests, contract matrix, dependency audit, CodeQL/workflow security.
- **INTEGRATION:** real SQLite/migrations, application ports, tools/sandbox/evidence, provider MockProvider streams.
- **E2E:** browser, native desktop, golden product flows, single/multi-agent, recovery, scheduler, external protected suites.
- **DEEP/NIGHTLY:** fuzz, property/mutation pilots, load, soak, chaos, full cross-platform, real-provider and release rehearsal.
- **RELEASE:** clean checkout, full relevant matrix, package/SBOM/provenance/signature, clean install, upgrade/rollback and artifact verification.

### Deterministic runtime strategy

`test-support` becomes the only owner of deterministic builders, IDs/seeds, virtual clock, MockProvider, virtual tools, fixture project directories and expected event/evidence assertions. Tests must not share filesystem paths, external ports, mutable clocks, real credentials or global state.

### External integration policy

Telegram, GitHub mutation, real provider, payment and other external suites run only in protected sandbox environments with least-privilege test credentials and dedicated fixtures. Missing credentials produce `NO_PROOF`, never PASS. Dangerous mutations against production repositories/accounts are prohibited.

### CI topology

- PR workflows orchestrate versioned local scripts; scripts own test behavior.
- Independent jobs run in parallel; only real data/identity dependencies use `needs`.
- PR fast/core/integration/browser E2E checks are stable named contexts.
- Nightly gathers full diagnostics instead of hiding later failures behind fail-fast.
- Release gates consume exact-SHA artifacts and run installation/upgrade/rollback smoke before promotion.

### Required check proposal

The actual branch ruleset must be inspected only after new workflows exist. Candidate stable contexts are: `Build Rust`, `Build Rust Windows`, `Build Frontend`, `Build Tauri Desktop`, `Frontend E2E Chromium`, `ONP SDD verify and audit`, `Quality integrity`, `w0-contract-gate`, `CodeQL (rust)`, `CodeQL (javascript-typescript)`, plus new contexts only after observed in GitHub.

## Golden scenarios

| ID | Scenario | Earliest gate |
|---|---|---|
| GOLDEN-001 | project creation/list/persistence | PR-356 |
| GOLDEN-002 | single-agent deterministic conversation | PR-357 |
| GOLDEN-003 | streaming cancellation | PR-357 |
| GOLDEN-004 | denied high-risk tool | PR-359 |
| GOLDEN-005 | checkpoint/recovery no duplicate effect | PR-362 |
| GOLDEN-006 | multi-agent collaboration | PR-361 |
| GOLDEN-007 | relevant memory retrieval | PR-360 |
| GOLDEN-008 | skill import/activate/use/rollback | PR-360 |
| GOLDEN-009 | development executor evidence/judge loop | PR-361 |
| GOLDEN-010 | human approval deny/expiry/resume | PR-359 |

## Global Definition of Done

The platform is complete only when each critical layer has deterministic tests, real integrations where required, automated main E2E and recovery evidence, explicit negative/security suite, clean-room CI, stable required checks, documented cross-platform evidence, release install/upgrade/rollback proof where claimed, useful failure artifacts, requirement-to-evidence traceability and no hidden manual prerequisite for a required gate.
