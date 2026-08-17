# Spec: W0 contract closure

> feature: w0-contract-closure
> status: em-implementacao

## Contexto

Como equipe responsável pela plataforma desktop multiagente Hank, precisamos transformar os cinco blockers W0 em contratos normativos verificáveis antes de permitir implementação de produto, para impedir que arquitetura, fila de PRs ou execução por agentes dependam de interpretação informal.

## Histórias

### US-001 — Fronteira independente do core

Como arquiteto do produto, quero uma fronteira explícita entre `agent-core`, adapters e shell Tauri, para que a lógica de domínio seja reutilizável por CLI, testes e superfícies não-Tauri.

#### AC-001 — Grafo de camadas e ownership

- **Dado** os módulos e crates descritos no SDD mestre e em `architecture-boundaries.md`
- **Quando** o manifesto arquitetural for validado
- **Então** cada camada terá responsabilidade, owner, dependências permitidas, processo/lifecycle e contratos de entrada/saída identificados, sem depender de inferência textual

#### AC-002 — Adapter não-Tauri exercita o mesmo caso de uso

- **Dado** um caso de uso da Application API definido sem dependência de UI
- **Quando** um adapter fake ou CLI o executar
- **Então** o caso de uso produzirá o mesmo contrato observável que o adapter Tauri, sem importar Tauri no core

#### AC-003 — Edges proibidas falham fechadas

- **Dado** um grafo que contenha import de Tauri/UI no core, regra de domínio no shell ou edge concreta substituindo uma port
- **Quando** o validator arquitetural rodar
- **Então** o resultado será `BLOCKED` ou `NO_PROOF`, nunca `PASS`

### US-002 — Ownership e dependências sem ambiguidade

Como implementador ou revisor, quero uma matriz única de ownership e um validator de edges/ciclos, para que nenhum card possa inventar owner, dependência ou lifecycle.

#### AC-004 — Matriz cobre comandos e crates

- **Dado** os crates, processos, comandos, eventos e adapters previstos na fila
- **Quando** a matriz de ownership for consultada
- **Então** cada item terá exatamente um owner e uma regra de dependência verificável

#### AC-005 — Ciclos e edges inválidas são rejeitados

- **Dado** uma dependência inexistente, um ciclo ou uma edge proibida inserida na fila/grafo
- **Quando** o validator for executado
- **Então** ele emitirá falha identificável com o item ofensivo e não produzirá estado aprovado

#### AC-006 — Lifecycle e compatibilidade estão definidos

- **Dado** cada boundary entre core, runtime, storage, event bus, provider e adapter
- **Quando** o contrato for revisado
- **Então** processo, lifecycle, threading/async, erros e compatibilidade de contrato estarão explícitos ou marcados como `NO_PROOF`

### US-003 — Fila de PRs e DAG mecanicamente auditáveis

Como coordenador do projeto, quero uma ficha canônica e um DAG validável para os 270 cards, para que a execução comece sempre no card correto e pare diante de qualquer inconsistência.

#### AC-007 — Os 270 cards têm schema completo

- **Dado** os três arquivos de fila e o índice mestre
- **Quando** o parser da fila for executado
- **Então** ele encontrará exatamente `PR-001` até `PR-270`, sem duplicatas, lacunas ou campos canônicos ausentes

#### AC-008 — Dependências e labels inválidos falham

- **Dado** um card com dependência inexistente, ciclo, label não normalizado ou arquivo provável tratado como existente
- **Quando** o validator rodar
- **Então** o card será marcado `BLOCKED`/`NO_PROOF` e o relatório nomeará a inconsistência

#### AC-009 — PR-001 e M16 são inequívocos

- **Dado** o requisito de inicializar pelo card `PR-001` e a decomposição da milestone M16
- **Quando** o índice e o DAG forem validados
- **Então** `PR-001` será único, M16 estará decomposta em cards numerados e o predecessor de cada card será resolvido

### US-004 — Execução por agentes com evidência e isolamento

Como mantenedor do projeto, quero um contrato operacional machine-readable para cada execução, para que agentes não alterem `main`, não escapem do escopo e não reutilizem evidência stale.

#### AC-010 — Preflight captura identidade e escopo

- **Dado** um card selecionado para execução
- **Quando** o preflight iniciar
- **Então** ele registrará branch, base SHA, tree SHA, estado dirty, card, non-goals, arquivos permitidos, comandos autorizados e owner da execução

#### AC-011 — Worktree, branch e path allowlist são impostos

- **Dado** uma tentativa de escrever em `main`, fora do worktree ou fora dos arquivos permitidos
- **Quando** o runner aplicar o contrato
- **Então** a operação será recusada antes da alteração e emitirá `BLOCKED`

#### AC-012 — Review independente e anti-self-approval

- **Dado** um artifact produzido por um agente
- **Quando** a etapa de review for avaliada
- **Então** o reviewer autenticado será distinto do autor e a aprovação será vinculada ao mesmo SHA/tree/policy

#### AC-013 — Evidência stale é invalidada

- **Dado** um rebase, mudança de base, alteração da policy ou mudança do tree SHA após a execução
- **Quando** o evidence manifest for revalidado
- **Então** a prova anterior será marcada inválida e o gate retornará `NO_PROOF` ou `BLOCKED`

### US-005 — Gate negativo e fechamento honesto

Como release/governance reviewer, quero fixtures adversariais e estados explícitos, para que documentação parcial nunca seja confundida com prova de implementação ou enforcement.

#### AC-014 — Fixtures negativas cobrem W0

- **Dado** fixtures para import proibido, ciclo, dependência ausente, branch main, scope drift, secret em artifact, reviewer igual ao autor e SHA stale
- **Quando** a suíte negativa rodar
- **Então** cada fixture falhará fechadamente e permanecerá rastreável a ARCH-001, ARCH-002, GOV-001, GOV-002 ou GOV-003

#### AC-015 — Gate produz estados machine-readable

- **Dado** uma entrada válida, uma entrada sem prova e uma entrada bloqueada
- **Quando** o gate runner processar cada caso
- **Então** produzirá exatamente um estado entre `PASS`, `NO_PROOF` e `BLOCKED`, com motivo e evidência identificáveis

#### AC-016 — Auditoria não declara resolução sem prova

- **Dado** que os artefatos normativos existam, mas não tenham execução vinculada a SHA/tree/policy
- **Quando** a auditoria do W0 for executada
- **Então** os blockers continuarão `PARTIAL/NO_PROOF` ou `OPEN/NO_PROOF`, sem liberar implementação downstream

## Fora de escopo

- Implementação do produto Rust/Tauri.
- Criação de crates, runtime, storage, provider, UI, workflows persistentes ou plugins.
- Execução de testes de produto, migrations, sandbox, assinatura, release ou deploy.
- Resolver os blockers W1–W4.
- Alterar silenciosamente o SDD, a fila de 270 cards ou o DAG existente.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-001 | A fila de 270 cards e o DAG versionados no commit atual são a fonte normativa de planejamento até uma reconciliação explícita. | aberta | — |
| ASM-002 | O primeiro fechamento W0 pode usar validators e fixtures documentais antes da implementação do runtime, desde que não seja rotulado como prova de produto. | aberta | — |
| ASM-003 | O repositório GitHub `stoltembergg-png/hank` permanecerá privado durante o planejamento. | aberta | — |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-001 | Qual identidade/provedor autenticado será o executor oficial do gate de PR e do reviewer independente? | respondida | GitHub Actions com token least-privilege executa os gates; reviewer deve ser uma identidade GitHub distinta do autor. Automação produz check, nunca aprovação. |
| Q-002 | Qual política GitHub final exigirá o check obrigatório quando o workflow de CI existir? | respondida | O check protegido será `w0-contract-gate`; ausência, falha, timeout ou identidade stale mantém `NO_MERGE`. A proteção só será ativada depois que o workflow e o contexto forem verificados ao vivo. |
| Q-003 | Quais formatos machine-readable serão normativos para queue schema, evidence manifest e architecture graph? | respondida | JSON Schema Draft 2020-12 valida queue card, evidence manifest e architecture graph; JSON carrega fixtures/provas; Markdown registra ADRs, decisões e critérios humanos. |
