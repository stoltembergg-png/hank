# Spec: Workflow execution

> feature: workflow-execution
> status: implementada

### US-963 — Coordenar execução bounded de workflow

Como runtime, quero coordenar a execução bounded de um DAG persistido para que a ordem,
os estados, cancelamento e falhas sejam determinísticos e observáveis sem acoplar handlers.

#### AC-964 — Run aceita somente DAG válido e expõe identidade bounded

- **Dado** um grafo válido, **Quando** um run é iniciado, **Então** ele recebe `run_id`, `workflow_id`, versão e capacidade máxima de nós em voo; identidade vazia, capacidade zero e DAG inválido são rejeitados antes de qualquer estado mutável.

#### AC-965 — Planner libera nós em ordem determinística e respeita backpressure

- **Dado** um DAG linear ou ramificado, **Quando** nós são despachados/concluídos, **Então** somente pré-requisitos concluídos liberam o próximo nó, a ordem é lexical/determinística e o limite de nós em voo rejeita despacho excedente sem mutação.

#### AC-966 — Outcomes são terminais, canceláveis e idempotência é fail-closed

- **Dado** um run ativo, **Quando** um node completa, falha, esgota retry ou o run é cancelado, **Então** os estados terminais são preservados; dispatch duplicado, conclusão duplicada e mutações após terminal são rejeitados sem reabrir o run.

## Suposições

- ASM-967: a execução desta PR coordena estados e envelopes; handlers concretos e efeitos externos pertencem às PRs seguintes.

## Perguntas em aberto

Nenhuma.
