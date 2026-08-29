# Spec: automation UI

> feature: automation-ui
> status: implementada

### US-1270 — Governar jobs agendados pela UI desktop

Como owner de um projeto, quero criar, editar, pausar e inspecionar jobs agendados pela UI
Desktop, para governar automações persistidas sem editar SQLite/configuração manualmente.

#### AC-1271 — CRUD bounded via application bridge
- **Dado** um projeto e um owner autorizados
- **Quando** a UI cria, lista ou atualiza um scheduled job
- **Então** a operação atravessa somente a application API/bridge Tauri, preserva project scope e não acessa SQLite diretamente.

#### AC-1272 — Triggers e targets explícitos
- **Dado** um job interval/cron/one-shot e um target versionado
- **Quando** o formulário é submetido
- **Então** tipos, valores, timezone, concurrency e missed-run policy são enviados explicitamente; valores inválidos falham antes da mutação.

#### AC-1273 — Lifecycle e revisão stale fail-closed
- **Dado** um job ativo ou disabled com uma revisão conhecida
- **Quando** o owner pausa, reativa ou salva uma edição stale
- **Então** pause/enable é explícito e revisão stale não sobrescreve o job.

#### AC-1274 — UI acessível e segura
- **Dado** a tela de automações
- **Quando** jobs são exibidos ou uma operação falha
- **Então** a lista é bounded, estados/next occurrence/erro são acessíveis, nenhum secret aparece e respostas stale não substituem estado mais novo.

## Fora de escopo

- worker/executor, scheduler loop, workflow/agent dispatch, histórico e notifications;
- acesso direto ao SQLite pelo frontend;
- auto-enable silencioso, privilege escalation, autonomous evolution e mutation dinâmica de workflow.

## Suposições

- ASM-1275: a autorização de owner/project permanece na application boundary existente e será representada por identidade bounded, sem inventar login nesta feature. Status: confirmada pelo contrato de Project/scheduler.

## Perguntas em aberto

Nenhuma.
