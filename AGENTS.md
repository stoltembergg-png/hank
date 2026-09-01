# AGENTS.md — Hank

Guia de entrada para agentes de coding, revisão e QA. Este arquivo não substitui as
políticas do projeto: em caso de conflito, as fontes canônicas abaixo vencem.

## Leia antes de trabalhar

- `AI_AGENT_GOVERNANCE.md` — autonomia, segurança, efeitos externos e evidência.
- `CONTRIBUTING.md` — fluxo de contribuição, testes e requisitos de PR.
- `.planning/master/agent-development-policy.md` — política normativa de agentes.
- `.planning/master/sdd-master.md` — decisões, escopo e non-goals do produto.
- `ARCHITECTURE.md` — explicação das fronteiras arquiteturais.
- `.planning/contracts/architecture-graph.json` — contrato arquitetural executável;
  seu schema e fixtures inválidas também fazem parte da autoridade.
- `.planning/master/dependency-dag.md` e `.planning/queue/queue-*.md` — predecessores,
  cards e condições de desbloqueio.
- `.github/required-checks.json` — manifesto dos required checks de `main`.

Não copie essas políticas para este arquivo. Aponte para a fonte original quando uma
regra já estiver documentada nela.

## Mapa mínimo

- `crates/` — workspace Rust: core, runtime, protocolos, segurança, providers,
  workflows e suporte de testes.
- `apps/desktop/` — shell Tauri; `frontend/` — React/TypeScript/Vite.
- `.spec/features/<feature>/` — `spec.md` e `tasks.md` de cada feature.
- `.planning/` — SDD, DAG, queue, contratos e revisões.
- `.github/workflows/` — CI/CD; `tools/ci/` e `tools/` — validadores e runners.
- `test/`, `crates/*/tests/`, `frontend/tests/`, `desktop-e2e/` — testes e contratos.
- `.spec/verification/` — artefatos regeneráveis; nunca são prova atual sem conferir
  SHA, tree e policy.

## Regras específicas do repositório

- A arquitetura deve ser conferida pelo contrato, não por este resumo:
  `node tools/w0-contract-validator.mjs architecture`.
- `agent-core` permanece portátil; Tauri, Tokio, SQLx, rede e providers concretos
  ficam fora dele. `agent-runtime` chama `application-api`, e não `agent-core`
  diretamente; adapters não podem contornar a API.
- Entradas passam por schema, identidade/ownership, capability, lifecycle, quota e
  policy. Efeitos de tool, processo, filesystem, rede, Python, plugin, MCP ou remoto
  usam as fronteiras de permissão/sandbox definidas nas fontes canônicas.
- Secrets não devem aparecer em código, logs, traces, fixtures, artifacts, `.env`,
  clipboard ou comentários. Redija qualquer valor sensível como `[REDACTED]`.
- Use branch e worktree isolados. Não altere `main`, branch de outro agente ou
  arquivos fora do escopo do card. Reconcile o estado vivo de Git/GitHub antes de
  editar; evidência de outro SHA é stale.
- Não marque planejamento, `pending`, `blocked`, `partial`, `stale`, timeout ou
  ausência de execução como `PASS`. Diferencie implementado, validado localmente,
  validado em CI e integrado.
- Não desabilite checks, reduza proteção, suprima findings ou faça merge prematuro.
  Auto-merge só pode integrar após required checks verdes no SHA atual e threads
  legítimas resolvidas.

## SDD/ONP

Para uma feature, preserve o conjunto correspondente de spec, tasks, documentação e
contratos/testes. O parser exige headings completos:

```text
### US-001 — Título da história
#### AC-001 — Título do critério
```

Use `Dado`, `Quando`, `Então`; mantenha IDs globais únicos; e use `@spec:AC-<id>`
quando a feature adotar tags. Uma task `[concluida]` precisa de evidência `PASS`.
Não edite a fila histórica para “corrigir” inconsistências: registre o problema no
DAG/contrato conforme a política normativa.

Verificação de uma feature:

```bash
node tools/ci/run-onp-spec.mjs verify <feature>
```

O bootstrap do ONP é checksum-pinned e falha fechado. Rode `audit --ci` somente como
parte do fluxo definido pela CI; não use JSON de verificação stale como prova.

## Gates locais focados

Escolha os gates exigidos pelo card e pelo impacto do diff; não execute builds globais
pesados sem necessidade. Nesta VPS, use `CARGO_BUILD_JOBS=1` e não rode
`cargo test --workspace` (o workspace já causou OOM).

```bash
# Rust
CARGO_BUILD_JOBS=1 cargo fmt --all -- --check
CARGO_BUILD_JOBS=1 cargo clippy --package <crate> --all-targets --locked -- -D warnings
CARGO_BUILD_JOBS=1 cargo test --package <crate> --locked

# Frontend, a partir de frontend/
npm ci --no-fund
npm run lint
npm run typecheck
npm test
npm run build
```

## Git, PR e handoff

- Commits seguem `.commitlintrc.json`: tipos permitidos e subject de até 72 caracteres.
- Use RED → GREEN → REFACTOR quando houver comportamento/teste novo.
- Uma PR deve ser pequena, rastreável a um card, declarar escopo/non-goals, testes,
  riscos e rollback, e conter exatamente `Plan card: PR-###` ou `Plan card: none`.
- Após rebase, push, mudança de base ou conclusão tardia de CI, revalide tudo no novo
  SHA/tree; invalide evidência anterior.
- O handoff deve listar mudança, arquivos impactados, comandos e resultados reais,
  SHA/tree, status dos required checks/threads (`PASS`, `FAIL`, `BLOCKED` ou `NO_PROOF`),
  riscos, rollback e blockers. Nunca inclua secrets.
