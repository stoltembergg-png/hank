# Spec: security tests

> feature: security-tests
> status: em-implementacao

## Contexto

O plano de produção hardening (M16) introduziu várias boundaries puras
(branch policy, mcp permission, plugin permission, rate limiting, resource
reservation, audit log) e exige uma camada de testes de regressão que
bloqueie regressão silenciosa entre boundaries. Sem uma matriz de ameaças
machine-readable e uma suíte fail-closed, o gate de segurança depende de
adesão humana.

## Histórias

### US-2100 — Garantir regressão negativa de segurança por matriz de ameaças e suíte fail-closed

Como runtime de segurança, quero uma matriz de ameaças versionada e uma
suíte de testes de regressão negativa que bloqueie qualquer TM-NNN do
manifest quando o fixture correspondente produzir comportamento de
bypass, vazamento, denial indevido, stale evidence ou metadado de release
inválido, para que regressão silenciosa entre boundaries seja
impossível.

#### AC-2101 — Manifest versionado de regressão de ameaças

- **Dado** um `threat-regression-manifest.json` em `docs/security/`
- **Quando** o runner o carregar
- **Então** o schema é validado, cada TM-NNN tem `id`, `boundary`, `threat`,
  `severity`, `fixture_id`, `expected_outcome`, `test_id` e `revision`; um
  TM-NNN desconhecido bloqueia; ordem de TMs é determinística.

#### AC-2102 — Malformed IPC e origin confusion bloqueiam

- **Dado** envelopes IPC com campos faltantes, tamanhos inconsistentes ou
  origens não allowlisted
- **Quando** o boundary correspondente processar
- **Então** a decisão é `deny` tipada, nenhum efeito externo é produzido e
  a chain de auditoria registra `denial` com o TM-NNN correspondente.

#### AC-2103 — Path traversal, credential leakage e plugin/remote denial bloqueiam

- **Dado** tentativas de path traversal via `..`, `~`, `/`, symlinks, paths
  brutos, valores de credencial em payload, plugin/remote sem autorização
- **Quando** a boundary testar
- **Então** a rejeição é tipada e o secret/token/senha/key/string-de-conexão
  nunca aparece no retorno serializado, log ou export.

#### AC-2104 — Stale evidence e bad release metadata bloqueiam

- **Dado** evidências com SHA diferente, tree divergente, policy revision
  stale, ou metadado de release com SHA inválido
- **Quando** o validator consumir
- **Então** o resultado é `stale_evidence` ou `invalid_release_metadata`
  tipado, o gate de release falha fechado, e nenhum merge/auto-merge é
  aprovado enquanto a stale evidence não for resolvida.

#### AC-2105 — Secret scanning, dependency/advisory baseline e SAST/negative CI-policy fixtures existem

- **Dado** artefatos gerados (logs, fixtures, exports, manifests) e
  contratos de CI
- **Quando** a suíte de segurança rodar
- **Então** secret scanning rejeita padrão `api_key|senha|password|token|secret
  :|="..."` em artefatos committed; o baseline de advisories não cresce sem
  nova entrada; o `required-checks.json` permanece coerente com o ruleset
  ativo; nenhum `continue-on-error: true` aparece em checks obrigatórios.

#### AC-2106 — Manifest ausente ou stale bloqueia

- **Dado** o manifest apagado, renomeado, com `revision` divergente do
  runner ou sem `test_id` mapeável
- **Quando** a suíte de segurança rodar
- **Então** a execução falha fechado com classificação explícita
  `MANIFEST_MISSING`, `MANIFEST_STALE` ou `MANIFEST_ORPHAN`; nenhum
  resultado parcial é aceito.

#### AC-2107 — Resultado é determinístico, redacted e versionado

- **Dado** o manifest, as TMs e os fixtures
- **Quando** a suíte produzir `result.json`
- **Então** o SHA256 do resultado é estável para o mesmo `tree_sha`,
  `policy_revision` e `runner_revision`; valores sensíveis são
  redacted a `[REDACTED]`; o runner é fail-closed e o artefacto é
  uploaded com `if-no-files-found: error`.

## Fora de escopo

- Penetration test em ambiente de produção.
- Mudar policy ou gate para fazer teste passar.
- Adicionar fixture sem TM-NNN correspondente no manifest.
- Marcar teste como `ignore` ou `#[ignore]` sem policy.
- Falsificar SHA de evidência ou policy.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-2101 | O manifest fica em `docs/security/threat-regression-manifest.json` e é a única fonte canônica de TM-NNN. | confirmada | codificada em AC-2101. |
| ASM-2102 | Os runners Rust vivem em `crates/security-core/tests/security_regression_contract.rs` e os Node em `tools/security/threat-regression.spec.mjs`; nenhum dos dois afirma ausência de vulnerabilidade. | confirmada | documentada no escopo e em `non-goals` (Não-escopo). |
| ASM-2103 | O workflow `ci-security.yml` é `pull_request` + `push: main` e roda somente em `ubuntu-24.04`, sem privilégios de escrita no repositório. | confirmada | registrada em `tools/security/README.md` (criado nesta fatia). |
| ASM-2104 | A execução do runner é bounded por tempo (≤10 min), por tamanho de output e por quantidade de TMs (≤128). | confirmada | os limites concretos ficam no `tools/security/threat-regression.mjs`. |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-2101 | Como o runner deve reagir quando um TM-NNN novo é adicionado sem tag `@spec:AC-21NN` no Rust ou no Node? | respondida | falha fechado com `MANIFEST_ORPHAN`; o runner é estrito. |
| Q-2102 | Onde a suíte negativa de CI-policy deve rejeitar `continue-on-error: true`? | respondida | em qualquer check dentro de `jobs:` em `.github/workflows/*.yml`, exceto steps de cleanup/notification explicitamente marcados. |
