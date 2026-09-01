# Spec: native evaluation runner

> feature: native-evaluation-runner
> status: auditada

## Contexto

O corpus nativo core já possui casos, fixtures e baselines versionados, mas
 ainda não há uma execução reproduzível que una esses elementos a uma
 identidade exata de ambiente. Este incremento adiciona somente um runner
 offline e provider-neutral para replay do baseline. Ele não chama providers,
 ferramentas, rede, secrets ou filesystem de produção.

### US-1453 — Executar o corpus em ambiente versionado

Como mantenedor do Harness, quero executar o corpus core em uma identidade
versionada de código, árvore, policy, schema e ambiente para produzir um
relatório de baseline auditável.

#### AC-1453 — Baseline PASS registra identidade e métricas

- **Dado** o corpus core e um ambiente compatível com seus baselines.
- **Quando** o runner executa o corpus em um workspace de fixtures controlado.
- **Então** cada relatório preserva `head_sha`, `tree_sha`, policy, schema,
  fixture, ambiente, artifacts, terminal e métricas, e o replay termina em
  `PASS` para os casos promocionáveis sem alterar seus estados não promocionáveis.

### US-1454 — Rejeitar ambientes incomparáveis

Como comparador, quero que o baseline só seja produzido quando a identidade
do ambiente coincidir exatamente com a referência congelada.

#### AC-1454 — SHA, árvore, policy, schema ou ambiente divergente falham fechado

- **Dado** um corpus com baseline vinculado a uma identidade conhecida.
- **Quando** qualquer parte da identidade observada diverge.
- **Então** o runner rejeita explicitamente o ambiente como incomparável e não
  produz um run parcialmente comparável.

### US-1455 — Impedir execução insegura ou evidência incompleta

Como mantenedor de segurança, quero que o runner valide tudo antes de
materializar fixtures e rejeite entradas incompletas, não determinísticas ou
com efeitos externos.

#### AC-1455 — Artifact ausente, fixture não determinística e efeito externo falham fechado

- **Dado** uma entrada com artifact requerido ausente, fixture não determinística
  ou `ExternalWrite` declarado.
- **Quando** o runner é chamado.
- **Então** ele retorna erro específico antes do replay correspondente e nunca
  chama provider, ferramenta, processo, rede ou filesystem de produção.

### US-1456 — Manter boundedness e replay idempotente

Como operador de CI, quero que o runner tenha limites explícitos e produza o
mesmo digest quando o mesmo corpus e ambiente forem reexecutados.

#### AC-1456 — Limites, saída e idempotência são verificáveis

- **Dado** um corpus dentro dos limites e uma configuração bounded.
- **Quando** o runner é executado uma ou mais vezes.
- **Então** a saída permanece limitada, o workspace não é sobrescrito e o
  `run_digest` permanece idêntico para a mesma entrada.

## Fora de escopo

- comparação candidate/baseline, seleção de cases, persistência ou UI;
- providers, rede, secrets, ferramentas e filesystem de produção;
- paralelismo não determinístico, efeitos externos e ativação automática;
- adapters externos, execução do modelo e publicação de release.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Definition of Done

Runner nativo provider-neutral, bounded e fail-closed, com replay de SHA exato
do corpus core, relatório de baseline com evidência completa, erros negativos
para ambiente incomparável/artifact ausente/nondeterminismo/efeito externo,
testes focais, documentação e verify/audit ONP passando no SHA exato.
