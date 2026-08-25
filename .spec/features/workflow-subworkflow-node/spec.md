# Spec: SubWorkflowNode plan

> feature: workflow-subworkflow-node
> status: em-implementacao

### US-1020 — Compor workflows versionados com isolamento bounded

Como executor de workflow, quero resolver uma referência versionada e mapear entradas para um
child run correlacionado, sem ciclos, runaway depth, orçamento excedido ou acesso cross-project
não autorizado.

#### AC-1021 — Resolução e mapping são determinísticos

- **Dado** um catálogo com versão registrada
- **Quando** a referência e o mapping são válidos
- **Então** o planner resolve a versão exata e produz child correlation e inputs ordenados.

#### AC-1022 — Escopo e limites falham fechado

- **Dado** versão ausente, mapping inválido, projeto divergente, ciclo, depth ou budget excedido
- **Quando** a composição é planejada
- **Então** a operação é rejeitada sem criar child run.

#### AC-1023 — Cancelamento e correlação são idempotentes

- **Dado** um child run planejado
- **Quando** o pai cancela ou a mesma correlação é solicitada novamente
- **Então** o child termina cancelado e a mesma correlação não cria duplicata.

## Suposições

- ASM-1024: persistência, leases e execução do child pertencem à PR-186; esta fatia mantém catálogo e plano bounded em memória.

## Perguntas em aberto

Nenhuma.
