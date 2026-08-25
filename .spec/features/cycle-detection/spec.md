# Spec: Cycle detection

> feature: cycle-detection
> status: implementada

## História de usuário

### US-895 — Bloquear ciclos de delegation antes da mutação

Como boundary de invocation, quero detectar self-loop e ciclos na ancestry,
para impedir consumo indefinido de budget/context/rounds antes de dispatch.

#### AC-897 — Self-loop é rejeitado

- **Dado** caller e callee idênticos
- **Quando** o preflight é executado
- **Então** retorna `RejectSelfLoop` sem alterar o grafo.

#### AC-898 — Ciclo ancestral é rejeitado e caminho acíclico passa

- **Dado** ancestry A→B→C
- **Quando** o candidato retorna a A ou segue para target novo
- **Então** o primeiro é rejeitado com path length e o segundo passa.

#### AC-899 — Grafo incompleto falha fechado e decisão é idempotente

- **Dado** parent ausente ou request inválido
- **Quando** o detector é consultado repetidamente
- **Então** rejeita sem mutar estado e retorna a mesma decisão.

## Fora de escopo

- Política de profundidade máxima, paralelismo, scheduler, provider, transport,
  persistência e UI.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
