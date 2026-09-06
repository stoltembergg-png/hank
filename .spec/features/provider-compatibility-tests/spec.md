# Spec: provider compatibility tests

> feature: provider-compatibility-tests
> status: implementada

## Contexto

Providers e adapters devem obedecer aos contratos normalizados sem depender de
credenciais, rede, disponibilidade externa ou adaptação silenciosa.

## Histórias

### US-2650 — Validar adapters contra contratos normalizados

Como mantenedor, quero uma matriz offline de providers para bloquear divergências
de streaming, capabilities, erros, fallback, usage/cost e redaction.

#### AC-2651 — Matriz de compatibilidade

- **Dado** fixtures determinísticos para seis boundaries de provider
- **Quando** a matriz de contratos é executada offline
- **Então** cada adapter é classificado como supported, bounded ou expected-fail e o resultado é reproduzível no SHA/tree do teste.

#### AC-2652 — Capability fail-closed

- **Dado** capability supported, unsupported ou unknown
- **Quando** uma request requer modality, feature ou limite
- **Então** somente supported satisfaz o contrato; unsupported e unknown falham explicitamente sem fallback implícito.

#### AC-2653 — Error, stream, retry e fallback

- **Dado** complete, stream, cancel, timeout, rate-limit, quota ou invalid request
- **Quando** o adapter normaliza o resultado
- **Então** status, erro, usage/cost, cancelamento e retryability preservam a política bounded e não criam loop de fallback.

#### AC-2654 — Redaction e isolamento

- **Dado** fixtures e metadados de provider
- **Quando** a evidência é serializada ou falha
- **Então** não há credencial, endpoint live, prompt sensível ou tráfego de rede e a identidade provider/model/fixture permanece verificável.

#### AC-2655 — Input de release fail-closed

- **Dado** incompatibilidade de contrato ou manifest inválido
- **Quando** o runner de compatibilidade é executado
- **Então** o gate falha fechado e não declara disponibilidade pública do provider.

## Fora de escopo

- Credenciais reais, endpoints públicos, disponibilidade de API ou tráfego de rede.
- Ranking de qualidade, benchmark de latência ou adaptação silenciosa.
- Assinatura e publicação de artefatos, pertencentes às PRs seguintes.

## Boundary de arquitetura

- `crates/provider-core` define capabilities, requests, responses, fallback e redaction.
- `crates/provider-adapters/*/tests` exercita os contratos de cada adapter existente.
- `docs/provider-compatibility-manifest.json` é a identidade canônica da matriz.
- `tools/security/provider-compatibility-feature-tests.mjs` executa a matriz offline.
- `.github/workflows/ci-provider-compatibility.yml` executa o gate dedicado.

## Modos de falha

- Manifest ausente, divergente, sem providers ou com network diferente de forbidden: falha fechado.
- Capability unsupported/unknown aceita como supported: falha.
- Teste skipped, credencial, endpoint ou rede real: falha sem declarar compatibilidade.

## Suposições

- Adapters concretos continuam atrás da boundary provider-core.
- Fixtures são sintéticas e não provam disponibilidade externa.

## Perguntas em aberto

Nenhuma.

## DoD

- Matriz, contrato, manifest, runner, workflow, step ONP e documentação presentes.
- Formatação, clippy, testes Rust, runner e verify ONP passam.
- Os cinco ACs têm prova TAP PASS.

## Unlocks

- PR-266 (release signing).
