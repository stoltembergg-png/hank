# Spec: Workflow edge and DAG validation

> feature: workflow-edge
> status: implementada

### US-953 — Validar arestas de workflow como DAG

Como workflow core, quero edges tipadas e validação determinística do grafo,
para impedir ciclos, referências ambíguas e conexões entre workflows.

#### AC-954 — DAG válido e bounded passa

- **Dado** um grafo com nodes e edges válidos
- **Quando** é validado
- **Então** o DAG passa de forma determinística e respeita limites de nodes/edges.

#### AC-955 — Topologia inválida falha com erro tipado

- **Dado** self-edge, node desconhecido, edge duplicada ou ciclo
- **Quando** o edge é adicionado ou o grafo é validado
- **Então** a operação falha sem executar expressões.

#### AC-956 — Isolamento de workflow é obrigatório

- **Dado** uma edge de outro workflow
- **Quando** é adicionada ao grafo
- **Então** a operação falha com diagnóstico de cross-workflow.

## Fora de escopo

- persistência, execução, paralelismo real, loops de produto e avaliação de condições;
- interpretação de condition labels como código.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
