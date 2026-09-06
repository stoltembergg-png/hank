# Spec: agent loop tests

> feature: agent-loop-tests
> status: implementada

## Contexto

Hank precisa de um contrato determinístico e bounded para validar o ciclo
provider→tool→próximo turno sem efeitos externos, impedindo loops infinitos,
duplicação de tools e avanço após eventos inválidos.

## Histórias

### US-2500 — Loop de agente determinístico e bounded

Como mantenedor, quero validar o ciclo de agente sob policy, budget e cancelamento
sem alegar qualidade de modelo ou disponibilidade de provider.

#### AC-2501 — Sucesso e trace determinístico

- **Dado** um roteiro sintético permitido
- **Quando** o loop executar a entrada
- **Então** termina em sucesso e produz o mesmo digest de trace para a mesma entrada.

#### AC-2502 — Idempotência de tool

- **Dado** um pedido de tool repetido
- **Quando** o loop reprocessar o pedido
- **Então** registra replay sem cobrar novamente nem duplicar o custo ou o evento de efeito modelado.

#### AC-2503 — Policy e limites

- **Dado** tool negada, ciclo, profundidade ou orçamento excedido
- **Quando** o loop avaliar o passo
- **Então** interrompe fail-closed sem avançar.

#### AC-2504 — Cancelamento e evento stale

- **Dado** cancelamento ou evento ausente
- **Quando** o loop receber o estado
- **Então** não executa novos passos e retorna estado terminal seguro.

#### AC-2505 — Limite de turns e política inválida

- **Dado** limite inválido ou roteiro maior que o bound
- **Quando** o loop iniciar
- **Então** rejeita a política ou termina no limite finito.

## Suposições

- O contrato é sintético e não afirma qualidade de modelo nem disponibilidade de provider.
- Efeitos reais de tools, rede, shell e credenciais permanecem fora do modelo.

## Perguntas em aberto

- Nenhuma.
