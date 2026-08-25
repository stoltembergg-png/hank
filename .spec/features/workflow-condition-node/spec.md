# Spec: ConditionNode evaluator

> feature: workflow-condition-node
> status: em-implementacao

### US-988 — Avaliar branching declarativo

Como executor de workflow, quero avaliar somente expressões tipadas e bounded para escolher
uma saída sem executar código arbitrário.

#### AC-989 — Parser aceita somente subset versionado

- **Dado** uma expressão no subset `$.path ==|!=|>|< literal`
- **Quando** ela é analisada
- **Então** recebe AST tipada; chamadas, funções, parênteses, injeção e sintaxe desconhecida são rejeitadas.

#### AC-990 — Avaliação é determinística e sem efeitos

- **Dado** um documento JSON
- **Quando** a AST é avaliada
- **Então** retorna true/false determinístico, sem rede, filesystem, memória ou mutação.

#### AC-991 — Limites e dados ausentes falham fechado

- **Dado** path desconhecido, profundidade excessiva, expressão oversized ou tipo incompatível
- **Quando** o evaluator roda
- **Então** retorna erro tipado e não escolhe branch.

## Suposições

- ASM-992: o contexto de avaliação é um JSON já autorizado pelo workflow runtime; o evaluator não busca dados externos.

## Perguntas em aberto

Nenhuma.
