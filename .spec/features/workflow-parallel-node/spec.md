# Spec: ParallelNode planning contract

> feature: workflow-parallel-node
> status: em-implementacao

### US-994 — Planejar fan-out/fan-in bounded

Como executor de workflow, quero planejar branches independentes com limites explícitos e
join determinístico, sem iniciar workers ou introduzir scheduler implícito.

#### AC-995 — Fan-out e concorrência são bounded

- **Dado** um conjunto de node IDs únicos
- **Quando** um plano paralelo é criado
- **Então** fan-out e concorrência válidos são aceitos; zero, excesso ou duplicata falham fechado.

#### AC-996 — Join determinístico

- **Dado** resultados por node ID
- **Quando** a política `all`, `any` ou `quorum` é aplicada
- **Então** os resultados são reordenados na ordem declarada e a decisão é determinística.

#### AC-997 — Falha parcial e cancelamento não executam efeitos

- **Dado** branch failed/cancelled ou cancelamento do plano
- **Quando** o join é aplicado
- **Então** a política retorna estado tipado, não cria tarefas e não deixa branches órfãos.

## Suposições

- ASM-998: execução real, scheduler e propagação para providers permanecem fora desta fatia.

## Perguntas em aberto

Nenhuma.
