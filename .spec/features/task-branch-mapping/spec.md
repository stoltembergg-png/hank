# Spec: Task-to-branch mapping

> feature: task-branch-mapping
> status: em-implementacao

## Contexto

PR-207 cria o contrato único que vincula uma task a projeto, repositório,
worktree, branch, execução do agente e eventual PR. O mapping deve impedir que
um resultado seja atribuído à task ou ao repositório errado, sobreviver a
restart e encaminhar divergências para reconciliação explícita.

A boundary de domínio permanece pura e determinística. O repositório SQLite
fica em `agent-runtime`, atrás da API do domínio. Nenhuma operação deste slice
cria commit, executa Git, concede capability, publica PR ou altera policy.

## História

### US-1317 — Identidade única de task e branch

Como runtime de desenvolvimento, quero registrar uma identidade única de task,
repositório, worktree, branch e execução, para impedir atribuição cruzada.

#### AC-1317 — Mapping único e project-scoped @spec:AC-1317

- **Dado** uma task, projeto, repositório, worktree, branch e execução válidos
- **Quando** registro o mapping no registry
- **Então** ele é aceito uma vez, listado deterministicamente e uma task ou worktree duplicado é rejeitado sem mutar o estado anterior

#### AC-1318 — Isolamento de projeto e identidade completa @spec:AC-1318

- **Dado** um mapping registrado em um projeto
- **Quando** consulto, altero ou persisto usando outro projeto, task, repositório, branch ou execução incompatível
- **Então** a operação falha fechado e nenhum mapping existente é exposto ou alterado

### US-1319 — Lifecycle explícito e reconciliação

Como runtime reiniciável, quero preservar o lifecycle e a observação do mapping,
para que detach, resume, rebind e divergências nunca sejam mutações implícitas.

#### AC-1319 — Detach, resume e rebind exigem vínculo explícito @spec:AC-1319

- **Dado** um mapping ativo ou destacado
- **Quando** executo detach, resume ou rebind com revisão, revision esperada e autorização compatíveis
- **Então** a transição é determinística, incrementa revision uma vez e rejeita replay, revision stale ou rebind sem autorização

#### AC-1320 — Divergência vai para reconcile sem efeito externo @spec:AC-1320

- **Dado** um mapping ativo e uma observação de repositório/worktree/branch
- **Quando** a observação diverge ou coincide com a identidade registrada
- **Então** divergência vira `reconcile_required` preservando razão/observação, coincidência permanece ativa e nenhuma operação Git ou capability é executada

### US-1321 — Persistência transacional e bounded

Como runtime que pode reiniciar, quero persistir o mapping e sua revisão,
correlation IDs e campos de reconciliação, para retomar sem perder identidade.

#### AC-1321 — Migração, roundtrip e bounds @spec:AC-1321

- **Dado** um banco limpo ou já migrado
- **Quando** executo as migrações, salvo e recarrego um mapping válido
- **Então** a migração é idempotente, o roundtrip preserva identidade/lifecycle/revision e entradas vazias, oversized, com controle ou project-crossing falham antes do efeito

## Fora de escopo

- Criar task UI, commits, push, force-push, merge ou publicação de PR
- Execução de Git, filesystem, processo, rede ou capability grant
- Perfil de coding/reviewer/QA/security agent (PR-208–PR-211)
- Rebind automático, restart recovery destrutivo ou resolução automática de conflito
- Alterar branch policy, Ruleset, credentials, secrets, release ou migrations fora deste mapping

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-1322 | `agent-runtime` é a boundary de persistência existente e pode expor um repository SQLite específico sem criar `storage-core` prematuramente. | confirmada | O runtime já contém `SqliteStorage`, migrações e repositories transacionais; o domínio permanece sem `sqlx`. |
| ASM-1323 | Repositório e worktree usam identificadores textuais bounded compatíveis com os registros já existentes de `ProjectGitRepo` e `WorktreeRequest`. | confirmada | O contrato valida os identificadores e mantém `ProjectId`, `TaskId`, `RunId` e `TraceId` tipados para as identidades canônicas. |

## Perguntas em aberto

Nenhuma.

## Segurança e observabilidade

- Toda operação exige `project_id`, task e correlation/trace identity válidos.
- A chave de unicidade é `(project_id, task_id)` e `(project_id, worktree_id)`.
- Revision compare-and-set impede replay e atualização stale.
- Policy revision é preservada e revalidada; mapping não concede Git capability.
- Observações armazenam somente metadados bounded, nunca conteúdo de arquivo ou payload de provider.
- Erros não incluem conteúdo bruto de observações nem secrets.

## Definition of Done

- Contrato puro, registry bounded e repository SQLite implementados nos limites declarados.
- Migração versionada, idempotente e project-scoped.
- Testes positivos, concorrência/uniqueness, lifecycle, restart/roundtrip, mismatch e bounds anotados para todos os critérios.
- Documentação do lifecycle, ownership, reconciliação e rollback.
- `verify task-branch-mapping` e `audit --ci` executados após a última alteração.
- Quality Gates locais e CI remoto no SHA exato passam sem mascarar falhas.
