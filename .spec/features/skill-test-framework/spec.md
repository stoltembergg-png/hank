# Spec: Skill test framework

> feature: skill-test-framework
> status: implementada

## Contexto

Esta primeira fatia fornece um harness determinístico para validar fixtures

declarativas de Skills sem ativar a Skill, executar scripts ou tocar recursos
do host. O harness é destinado a testes de contrato e validação futura.

## Histórias

### US-645 — Executar uma fixture declarativa de Skill com limites

Como mantenedor de Skills, quero executar uma fixture bounded e reproduzível,
para validar o contrato sem conceder autoridade de runtime ao conteúdo testado.

#### AC-791 — Fixture válida produz resultado determinístico

- **Dado** um manifesto de teste válido com projeto, Skill, versão, trace e limite de passos
- **Quando** o harness executa a fixture declarativa
- **Então** produz resultado PASS bounded, com identidade e trace, sem executar scripts ou acessar o host

#### AC-792 — Fixture inválida ou perigosa falha fechada

- **Dado** uma fixture malformada, com override de instruções, script, rede ou limite inválido
- **Quando** o harness tenta iniciar a execução
- **Então** rejeita antes de qualquer efeito e informa uma razão estável sem conteúdo sensível

#### AC-793 — Reexecução preserva identidade e relatório

- **Dado** o mesmo manifesto e a mesma fixture
- **Quando** o harness executa a fixture novamente
- **Então** o resultado mantém o mesmo caso, identidade, limite e resumo redigido, sem mutação do estado ativo

## Fora de escopo

- Ativação, promoção, publicação ou alteração de versão ativa.
- Execução de scripts, processos, rede, filesystem real ou dependências externas.
- Integração de UI, SQLite, secrets ou provider real.
- Loop orchestration, sandbox OS e evaluator de candidatos, que ficam nas próximas fatias.

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-794 | Fixtures da primeira fatia são dados declarativos e não possuem callbacks executáveis. | confirmada | O modelo aceita somente passos declarativos bounded. |
| ASM-795 | A identidade de projeto, Skill e trace vem do chamador confiável. | confirmada | O manifesto valida formato e limites, mas não concede capabilities. |

## Perguntas em aberto

Nenhuma.
