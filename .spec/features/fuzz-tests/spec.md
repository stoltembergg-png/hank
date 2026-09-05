# Feature — Fuzz tests (M16)

**ID:** PR-261
**Milestone:** M16 — Production Hardening
**Categoria:** TESTING
**Status:** SPECIFICADO

## User Story

**US-2200 — Bounded fuzz harness for parser/state/permission boundaries**

> Como mantenedor, eu quero um lane de fuzz offline e reproduzível que persista
> seed, corpus, digest do corpus, digest do runner, SHA da tree e resultado por
> target, para que regressões em parsers e state machines em security-core,
> agent-protocol, workflow-core, recovery-core, secrets-core e migration-core
> sejam detectadas com artefato reproduzível, sem dependência de toolchains
> nightly, sem exfiltração de credenciais, e sem macular os deterministic
> tests já existentes.

## Acceptance Criteria

### AC-2201 — Manifest bound and versioned

**Dado** o arquivo `docs/security/fuzz-manifest.json`
**Quando** o runner inicia qualquer target
**Então** ele deve ler o manifest, recusar a iniciar se o manifest estiver
ausente, se o `schema_version` for diferente de 1, ou se o `manifest_revision`
não coincidir com o registrado no runner. O manifest é a única fonte
canônica de FT-001..FT-007.

### AC-2202 — Targets enumerated and registered

**Dado** o manifest com `targets[].id` em `FT-NNN`
**Quando** o runner enumera os targets
**Então** cada FT-NNN deve ter um `FuzzTarget` Rust registrado em
`crates/test-support/src/fuzz.rs`, com `name`, `kind` (`envelope|policy|state|
permission|release_metadata|hash_chain|rate_limit`), `parser` (função
pura), `invariants` (lista de predicates) e `smoke_iterations` (≥ 8).

### AC-2203 — Reproducible seed and corpus

**Dado** um target FT-NNN com seed `S` e corpus `C` determinístico
**Quando** o harness executa o target
**Então** o resultado deve ser reproduzível bit a bit para o mesmo
`(S, C, tree_sha, head_sha, runner_revision, toolchain_sha)`, e o runner
deve persistir `seed`, `corpus_digest` (SHA-256) e `runner_digest` em
`security/reports/fuzz-{target}.json`.

### AC-2204 — Crash artifact reproduces

**Dado** um panic capturado em uma iteração `i` com seed `S` e input `I`
**Quando** o harness re-executa com `seed=S`, `iterations=i+1` e
`replay=path/to/crash.bin`
**Então** o panic deve reproduzir no mesmo input, e o crash artifact deve
conter `target_id`, `seed`, `iterations`, `panicking_function`,
`input_digest` (SHA-256) e `stack_digest` (SHA-256 do backtrace textual).
Sem panic capturado → `panics=0`, `crash_artifact=null`.

### AC-2205 — Bounded resource/time limits

**Dado** qualquer execução de target
**Quando** o harness inicia uma iteração
**Então** o tempo total por target deve respeitar `smoke_iterations ×
per_iter_timeout_ms` (default 250 ms/iter), e a memória alocada não pode
ultrapassar `max_memory_mb` (default 64 MB/target). Exceder qualquer limite
resulta em `oom_count++` ou `timeout_count++` e o teste falha.

### AC-2206 — Runner output is single TAP

**Dado** o runner Node + Rust em `tools/security/fuzz-runner.mjs` e
`crates/test-support/tests/fuzz_contract.rs`
**Quando** o ONP `verify fuzz-tests` é executado
**Então** ele deve ler o TAP unificado com tags `@spec:AC-NNNN` e fechar
cada AC como PASS. O TAP nunca pode omitir testes; testes skipped ou
not_run contam como FAILED.

### AC-2207 — No credentials or unsafe corpus in repo

**Dado** o corpus commitado em `crates/test-support/fuzz-corpus/FT-*/`
**Quando** o runner valida o corpus no boot
**Então** o corpus deve ser unicamente strings sintéticas (UUIDs v4
gerados, JSON de fixtures internas, bytes aleatórios com seed); o runner
deve falhar closed se detectar pattern de secret conhecido
(AllowlistPatternId `NEG-001` da PR-260). O runner não carrega secrets
de ambiente e o digest do runner é a única coisa bind ao target.

## Out of Scope

- Real `cargo-fuzz` com libFuzzer (requer nightly).
- Mutation-based fuzzing sobre produção.
- Penetration testing.
- Discover new vulnerabilities.
- Disabling deterministic tests.
- Re-architecture of any boundary.
- Live network, live provider, live credentials.
- Claiming absence of bugs.

## Architecture Boundary

- `crates/test-support/src/fuzz.rs` — public surface: `FuzzTarget` trait,
  `FuzzHarness`, `FuzzOutcome`, `FuzzReport`, `FuzzCrash`, `FuzzLimits`,
  `run_target`, `run_all_targets`, `digest_corpus`, `digest_runner`,
  `replay_crash`, `verify_no_secrets`. No I/O outside workspace; no panic
  recovery hides errors; no `unsafe`.
- `crates/test-support/tests/fuzz_contract.rs` — `@spec:AC-2201..AC-2207`
  tests binding the Rust harness.
- `tools/security/fuzz-runner.mjs` — Node runner, no I/O outside the
  workspace, refuses to start without manifest, refuses to start without
  runner source, refuses to start if runner_digest mismatches the manifest.
- `docs/security/fuzz-manifest.json` — canonical manifest, schema_version 1.
- `.github/workflows/ci-fuzz.yml` — pull_request + push main, ubuntu-24.04,
  permissions: contents: read, runs smoke only, fails closed on panic/OOM.
- `onpspec.config.json` — registers `fuzz-tests` command with `tap` reporter.
- `.github/workflows/onp-sdd-evidence.yml` — `Verify fuzz tests` step.
- `.spec/features/fuzz-tests/tasks.md` — T-2200.
- `docs/fuzz-tests.md` — public surface, limits, ownership, runbook.
- `tools/security/README.md` — add fuzz lane entry.

## Failure Mode

- Manifest missing → exit 1 with `code=MISSING_MANIFEST`.
- Runner source missing → exit 1 with `code=MISSING_RUNNER`.
- runner_digest mismatch → exit 1 with `code=RUNNER_DIGEST_MISMATCH`.
- Corpus contains secret pattern → exit 1 with `code=CORPUS_SECRET_DETECTED`.
- Target panic → exit 1 with `code=PANIC`; crash_artifact path printed.
- Target OOM → exit 1 with `code=OOM`.
- Target timeout → exit 1 with `code=TIMEOUT`.
- Stack digest not reproducible → exit 1 with `code=NON_REPRODUCIBLE`.
- TAP missing `ok N - <name>` for an FT-NNN → ONP verify reports
  `AC_SEM_TESTE`.

## DoD

- Manifest, runner, harness, smoke tests, ONP step, CI workflow, docs all
  in repo.
- Local gates (cargo fmt, clippy, test, ONP verify fuzz-tests) all PASS.
- 7/7 ACs closed in TAP.
- No `unsafe` in harness code.
- No real network, no real provider, no real credentials.
- 0 secrets in corpus.

## Unlocks

- PR-262 (load tests), PR-263 (workflow recovery tests), PR-264 (agent
  loop tests), PR-265 (provider compatibility tests).
