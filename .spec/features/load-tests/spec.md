# Spec: load tests

> feature: load-tests
> status: implementada

## Contexto

Hank precisa de um contrato de load smoke bounded e reproduzível para medir
admission, fila, backpressure, cancelamento e identidade de artefato sem
alegar capacidade de produção ou coletar métricas do host.

## Histórias

### US-2300 — Medir comportamento bounded sob carga sintética

Como mantenedor, quero perfis S/M/L com workload versionado, seed e digest para
provar comportamento de admissão e backpressure sem tráfego real.

#### AC-2301 — Manifest bounded e versionado

- **Dado** o arquivo `docs/performance/load-manifest.json`
- **Quando** o runner iniciar
- **Então** ele deve validar revisão, seed, perfis S/M/L, warmup/repetições bounded, digest e flags de ausência de host metrics, produção e credenciais.

#### AC-2302 — Admission e backpressure bounded

- **Dado** um perfil cujo workload excede a capacidade declarada
- **Quando** o modelo executar
- **Então** `admitted + rejected == requests`, `max_in_flight <= concurrency` e `peak_queue <= queue`.

#### AC-2303 — Cancelamento contabilizado

- **Dado** um cenário bounded com cancelamento
- **Quando** o modelo concluir
- **Então** `completed + cancelled == admitted` e a duração declarada permanece dentro do limite do perfil.

#### AC-2304 — Repetição determinística e redaction

- **Dado** o mesmo manifest, seed e fixture sintética
- **Quando** a execução repetir
- **Então** as métricas e digests devem ser idênticos e não conter credenciais ou PII.

#### AC-2305 — Manifest inválido falha fechado

- **Dado** um manifest inválido ou sem perfis
- **Quando** o runner solicitar execução
- **Então** nenhuma carga deve executar e a operação deve falhar fechado.

## Fora de escopo

- Sampling de CPU, memória, disco, handles ou processos do host.
- Providers, internet, tráfego de produção ou credenciais.
- Ratificação de budgets de capacidade antes de baseline aprovado.

## Boundary de arquitetura

- `crates/test-support/src/load.rs` fornece o modelo puro, bounded e determinístico.
- `crates/test-support/tests/load_contract.rs` prova AC-2301..AC-2305.
- `docs/performance/load-manifest.json` é a identidade canônica do workload.
- `tools/security/load-runner.mjs` valida o manifest e grava receipt sem timestamp.
- `tools/security/load-feature-tests.mjs` emite TAP para `verify load-tests`.
- `.github/workflows/ci-load.yml` executa o receipt e o contrato.
- `.github/workflows/onp-sdd-evidence.yml` executa `verify load-tests`.

## Modos de falha

- Manifest ausente, divergente ou sem limites: falha fechado.
- Invariante de admissão, fila, cancelamento ou digest divergente: falha.
- Falta de perfil ou teste skipped: falha sem declarar capacidade.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## DoD

- Manifest, modelo, contrato, runner, workflow, step ONP e documentação presentes.
- Formatação, clippy, testes Rust, runner e verify ONP passam.
- Os cinco ACs têm prova TAP PASS.

## Unlocks

- PR-263 (workflow recovery tests).
