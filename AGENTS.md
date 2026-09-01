# AGENTS.md — Hank

Este arquivo orienta agentes de coding, revisão e QA que trabalham neste repositório.
Ele é um mapa operacional, não substitui as fontes normativas abaixo. Em conflito,
a fonte canônica vence.

## Fontes de verdade

Leia antes de alterar código ou workflows:

1. `AI_AGENT_GOVERNANCE.md` — regras mínimas de autonomia, efeitos externos,
   evidência e blockers.
2. `CONTRIBUTING.md` — ciclo de desenvolvimento, requisitos de PR e traceabilidade.
3. `.planning/master/agent-development-policy.md` — política normativa completa
   para agentes: preflight, escopo, segurança, testes, handoff e critérios de parada.
4. `.planning/master/sdd-master.md` — decisões de produto/arquitetura e non-goals.
5. `ARCHITECTURE.md` e `.planning/contracts/architecture-graph.json` — fronteiras
   arquiteturais e contrato executável.
6. `.planning/queue/queue-*.md`, `.planning/master/dependency-dag.md` e o card
   aplicável — escopo, predecessores e condição de desbloqueio.

Não duplique, reinterprete ou enfraqueça essas políticas neste arquivo. Se a tarefa
contradiz uma fonte normativa, pare e registre o blocker.

## Mapa do repositório

- `crates/` — workspace Rust; core, runtime, protocolos, segurança, providers,
  workflows e suporte de testes.
- `apps/desktop/` — shell Tauri.
- `frontend/` — React/TypeScript/Vite.
- `.spec/features/<feature>/` — `spec.md` e `tasks.md` por feature.
- `.spec/verification/` — evidência produzida pelo ONP; não a trate como prova
  atual sem verificar o SHA.
- `.planning/` — SDD, queue, DAG, ADRs, contratos e revisões.
- `.github/workflows/` — CI/CD; `.github/required-checks.json` é o manifesto de
  checks requerido para `main`.
- `tools/ci/run-onp-spec.mjs` — bootstrap checksum-pinned do ONP.
- `test/`, `crates/*/tests/`, `frontend/tests/`, `desktop-e2e/` — testes e contratos.

## Arquitetura: fronteiras que não podem ser furadas

A arquitetura normativamente verificável está em
`.planning/contracts/architecture-graph.json`; valide-a com:

```bash
node tools/w0-contract-validator.mjs architecture
```

Resumo de camadas:

```text
Presentation (React / Tauri / CLI / fake)
        -> Application API -> Domain/Core
Execution/Runtime -> Application API
Infrastructure adapters -> Domain/Core
```

- `agent-core` contém regras de domínio, invariantes, tipos e ports. Não depende de
  Tauri, Tokio, SQLx, rede ou providers concretos.
- Frontend/Tauri não acessam SQLite, filesystem, providers, tools ou secrets
  diretamente.
- Efeitos de tool, processo, filesystem, rede, Python, plugin, MCP ou remoto passam
  por Permission Engine e Sandbox/Execution Broker; default é deny/fail-closed.
- Nunca exponha secrets em código, logs, traces, fixtures, artifacts, `.env`,
  clipboard ou comentários. Use referências/handles redigidos.

Mudanças em schema, API/evento/trace, trust boundary, permission, workflow,
dependência, migration ou release exigem testes e atualização da documentação/ADR
aplicável, com impacto e rollback explícitos.

## Antes de editar

1. Reconcilie o estado vivo: `git status`, branch/base SHA, PRs/checks e card da
   queue. Não use resumos antigos como fonte de verdade.
2. Trabalhe em branch e worktree exclusivos. Não altere `main`, a branch de outro
   agente ou arquivos fora do card.
3. Confirme predecessores integrados e evidência no SHA correto. `planned`,
   `NO_PROOF`, `blocked`, `partial`, `stale`, timeout ou SHA/tree divergente não
   satisfazem dependência.
4. Declare objetivo, non-goals, acceptance criteria, arquivos/crates esperados,
   risco de segurança, testes e rollback antes da implementação.
5. Use RED → GREEN → REFACTOR: primeiro um teste/fixture focado que falha, depois a
   menor mudança compatível, então limpeza sem ampliar o escopo.

Pare em vez de improvisar quando faltar credencial/permissão, houver decisão de
produto/arquitetura, migration sem rollback comprovado, dependência sem evidência,
CI/security/architecture falhando, ou informação indispensável indisponível.

## Spec-driven development e ONP

Para cada feature, mantenha:

```text
.spec/features/<feature>/spec.md
.spec/features/<feature>/tasks.md
docs/<feature>.md                 # quando a feature tiver documentação de uso/contrato
crates/<crate>/tests/...           # testes de contrato quando aplicável
```

Convenções observadas pelo parser ONP:

- Use `### US-<id>` para user stories e `#### AC-<id>` para critérios de aceite.
- Critérios devem ter `Dado`, `Quando`, `Então` e testes rastreáveis por `@spec:AC-<id>`
  quando o padrão da feature usar tags.
- Mantenha IDs globais únicos; não recicle IDs já usados por outra feature.
- Uma tarefa `[concluida]` precisa de prova `PASS`; caso contrário mantenha-a pendente
  ou trate como blocker.

Verificação local da feature:

```bash
node tools/ci/run-onp-spec.mjs verify <feature>
```

O ONP falha fechado se o bootstrap/manifests tiverem checksum incompatível. A CI
executa os verifies antes do audit e regenera artefatos de verificação no SHA exato;
não edite ou use um JSON de verificação stale como prova de sucesso.

## Comandos locais usuais

Use gates focados no escopo. Não substitua resultados por intenção.

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --package <crate> --all-targets --locked -- -D warnings
cargo test --package <crate> --locked

# Arquitetura
node tools/w0-contract-validator.mjs architecture

# ONP
node tools/ci/run-onp-spec.mjs verify <feature>
node tools/ci/run-onp-spec.mjs audit --ci

# Frontend (em frontend/)
npm ci --no-fund
npm run lint
npm run typecheck
npm test
npm run build
```

Não rode suites globais pesadas apenas por hábito; selecione os gates exigidos pelo
card e pelo impacto do diff. Nunca desabilite, afrouxe ou marque como opcional um
gate para obter verde.

## Git, PR e CI

- Commits seguem `.commitlintrc.json`: tipos permitidos
  `build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test`; subject tem no
  máximo 72 caracteres.
- PRs devem ser pequenas, rastreáveis a um único card e declarar escopo, non-goals,
  critérios, testes reais, riscos e rollback.
- Inclua na descrição exatamente uma linha de rastreabilidade:
  `Plan card: PR-###` ou `Plan card: none`.
- Revalide depois de rebase, push, CI tardio ou mudança de base; evidência anterior
  fica stale quando o SHA/tree muda.
- Não afirme `PASS` para check pendente, falho, skipped inesperado ou executado em
  SHA diferente. Diferencie: implementado, validado localmente, validado em CI e
  integrado.
- Auto-merge depende de todos os required checks no SHA atual e de conversas de
  review resolvidas. Não force merge, não contorne proteção de branch e não resolva
  finding sem corrigir/justificar tecnicamente o código afetado.

## Relatório/handoff obrigatório

Reporte fatos verificáveis:

- o que mudou e o que deliberadamente não mudou;
- arquivos/crates, schema/migration/ADR/documentação impactados;
- comandos literais executados e resultado real, vinculados ao SHA/tree;
- CI/required checks e threads de revisão: `PASS`, `FAIL`, `BLOCKED` ou `NO_PROOF`;
- riscos, rollback, dependências desbloqueadas e blockers remanescentes.

Não inclua secrets ou dados sensíveis no handoff.
