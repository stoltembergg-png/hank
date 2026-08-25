# Spec: Maximum invocation depth

> feature: invocation-depth
> status: implementada

## História de usuário

### US-899 — Limitar profundidade de delegation

Como boundary de invocation, quero validar depth contra ancestry e limite
configurado, para impedir árvores profundas mesmo quando acíclicas.

#### AC-901 — Depth root e limite válido passam

- **Dado** request root com depth 0 e máximo positivo
- **Quando** preflight é executado
- **Então** passa com depth calculado deterministicamente; mismatch é negado.

#### AC-902 — Excesso e ancestry ausente falham fechado

- **Dado** depth acima do máximo ou parent ausente
- **Quando** o limite é consultado
- **Então** retorna rejeição sem mutar o grafo.

#### AC-903 — Repetição não cresce depth

- **Dado** o mesmo request e grafo
- **Quando** o preflight é repetido
- **Então** a decisão é idempotente e o grafo permanece inalterado.

## Fora de escopo

- Paralelismo, rounds, scheduler, provider, transport e UI.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.
