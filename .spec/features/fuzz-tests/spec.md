# Spec: fuzz tests

> feature: fuzz-tests
> status: implementada

## Contexto

O runtime Hank precisa de uma lane de fuzz bounded, offline no sentido de não
usar providers, rede de produção ou credenciais, e reproduzível para testar
boundaries de parser, estado e permissão sem substituir os testes
determinísticos existentes.

## Histórias

### US-2200 — Executar fuzz bounded e reproduzível nas boundaries de segurança

Como mantenedor, quero uma lane de fuzz bounded e reproduzível que registre
seed, corpus, digest do corpus, digest do runner, SHA da tree e resultado por
target, para detectar regressões em parser e state machine com artefato
reexecutável e sem exfiltração de credenciais.

#### AC-2201 — Manifest bounded e versionado

- **Dado** o arquivo `docs/security/fuzz-manifest.json`
- **Quando** o runner iniciar qualquer target
- **Então** ele deve ler o manifest e recusar a execução se o manifest estiver ausente, se `schema_version` for diferente de 1 ou se `manifest_revision` não coincidir com a revisão registrada pelo runner; o manifest é a fonte canônica de FT-001..FT-007.

#### AC-2202 — Targets enumerados e registrados

- **Dado** um manifest com `targets[].id` no formato `FT-NNN`
- **Quando** o runner enumerar os targets
- **Então** cada FT-NNN deve corresponder a um `FuzzTarget` Rust registrado em `crates/test-support/src/fuzz_targets.rs` e validado contra o registry, com nome, kind canônico, parser puro, invariants e `smoke_iterations` maior ou igual a 8.

#### AC-2203 — Seed e corpus reproduzíveis

- **Dado** um target FT-NNN com seed S e corpus C determinísticos
- **Quando** o harness executar o target
- **Então** o resultado deve ser reproduzível para o mesmo seed, corpus, tree SHA, head SHA, revisão do runner e toolchain, persistindo `seed`, `corpus_digest` e `runner_digest` no relatório.

#### AC-2204 — Crash artifact reproduzível

- **Dado** um panic capturado na iteração i com seed S e input I
- **Quando** o harness reexecutar com `seed=S`, `iterations=i+1` e um crash artifact
- **Então** o panic deve reproduzir no mesmo input e o artifact deve conter target, seed, iterações, função, digest do input e digest da stack; sem panic, `panics=0` e `crash_artifact=null`.

#### AC-2205 — Limites de recurso e tempo são bounded

- **Dado** qualquer execução de target
- **Quando** o harness iniciar uma iteração
- **Então** cada iteração retornada deve ser comparada a `per_iter_timeout_ms`, a entrada deve respeitar o orçamento de alocação derivado de `max_memory_mb`, e excedentes devem resultar em timeout ou OOM explícito; o contrato não mede memória do host nem promete interromper um target que nunca retorna, que fica limitado pelo timeout bounded do job CI.

#### AC-2206 — Saída do runner é TAP único

- **Dado** o runner Node e o contrato Rust em `tools/security/fuzz-runner.mjs` e `crates/test-support/tests/fuzz_contract.rs`
- **Quando** `verify fuzz-tests` for executado
- **Então** a saída deve conter TAP unificado com tags `@spec:AC-NNNN`, sem omitir testes; testes skipped ou not_run contam como falha.

#### AC-2207 — Corpus sem credenciais ou conteúdo inseguro

- **Dado** o corpus sintético usado pelos targets
- **Quando** o runner validar o corpus no boot
- **Então** ele deve falhar fechado ao detectar padrão de secret conhecido, não ler credenciais do ambiente e aceitar somente dados sintéticos/redacted.

## Fora de escopo

- `cargo-fuzz` com libFuzzer ou toolchain nightly.
- Mutation-based fuzzing sobre produção.
- Penetration testing ou descoberta de vulnerabilidades.
- Re-architecture de qualquer boundary.
- Rede de produção, providers ativos ou credenciais reais.
- Alegar ausência de bugs.

## Boundary de arquitetura

- `crates/test-support/src/fuzz.rs` fornece o harness bounded, digest,
  replay e verificação de corpus, sem `unsafe` e sem I/O externo.
- `crates/test-support/src/fuzz_targets.rs` registra FT-001..FT-007.
- `crates/test-support/tests/fuzz_contract.rs` prova AC-2201..AC-2207 e as
  regressões de zero iterations, replay e cobertura de kinds.
- `tools/security/fuzz-runner.mjs` valida o manifest, executa o contrato Rust
  e grava `security/reports/fuzz.json`.
- `docs/security/fuzz-manifest.json` é a fonte canônica dos targets.
- `.github/workflows/ci-fuzz.yml` executa apenas o smoke bounded em Ubuntu 24.04
  com `contents: read`.
- `.github/workflows/onp-sdd-evidence.yml` executa `verify fuzz-tests`.
- `onpspec.config.json` registra o comando e o reporter TAP.

## Modos de falha

- Manifest ausente ou inválido: falha com classificação explícita.
- Digest ou revisão incompatível: falha fechado.
- Corpus com padrão sensível: falha com `CORPUS_SECRET_DETECTED`.
- Panic, OOM ou timeout: falha com artifact e classificação correspondente.
- TAP incompleto: o verify reporta o AC sem prova.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## DoD

- Manifest, runner, harness, targets, contrato, workflow, step ONP e docs estão no repositório.
- Formatação, clippy, testes Rust, testes Node, verify ONP e actionlint passam.
- Os sete ACs têm prova TAP PASS.
- Não há `unsafe`, rede de produção ou credenciais reais no harness/corpus.

## Unlocks

- PR-262 (load tests), PR-263 (workflow recovery tests), PR-264 (agent loop tests) e PR-265 (provider compatibility tests).
