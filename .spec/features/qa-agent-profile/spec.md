# Spec: QA agent profile

> feature: qa-agent-profile
> status: auditada

## Contexto

PR-210 define o contrato de um agente QA que seleciona comandos de teste
permitidos e transforma resultados reais em evidência bound ao snapshot exato
do repositório. A execução concreta permanece em adapters/runtime autorizados;
o domínio apenas valida o plano, os resultados e o handoff.

## Boundary e não-escopo

- `agent-core` contém somente schema, allowlist, validação e decisões
  determinísticas.
- O profile não executa processos, shell, Git, filesystem, rede, provider ou
  CI e não interpreta texto como comando.
- Comandos são variantes tipadas; variantes `Shell`/`Arbitrary` são dados
  explicitamente rejeitados, nunca uma rota de execução.
- Logs brutos, prompts, segredos e payloads externos não são armazenados; o
  contrato conserva somente status, duração, tentativa e digests bounded.
- O QA report é input de evidência e failure handoff advisory; não desativa
  testes, muda expectations, cria PASS artificial nem decide release.

## Histórias

### US-1328 — QA plan allowlisted

Como executor autorizado, quero receber somente um plano de comandos de teste
conhecidos, bounded por timeout, tentativas, output e quantidade.

#### AC-1328 — Allowlist e limites

- **Dado** um profile válido e um mapping `Active`.
- **Quando** o plano referencia exatamente projeto, task, repository, worktree,
  branch, policy revision e comandos tipados allowlisted.
- **Então** a autorização retorna um permit bounded sem autoridade de alterar
  checks, expectations ou release.
- **Dado** comando shell/arbitrário, texto instruction-like, scope diferente ou
  profile/plano fora dos limites.
- **Quando** o plano é autorizado.
- **Então** falha fechado antes de qualquer execução.

### US-1329 — QA evidence bound

Como orquestrador, quero distinguir resultado passado, falho, skipped,
no-run, timeout, malformed e stale, vinculando cada resultado ao SHA/tree e ao
digest do artefato.

#### AC-1329 — Identidade e completude

- **Dado** um resultado para cada comando planejado, com SHA/tree exatos,
  output digest, artifact digest e status executado.
- **Quando** o report é validado contra profile e mapping.
- **Então** ele é `Complete` somente quando todos os resultados são `Passed`.
- **Dado** SHA/tree incorreto, resultado ausente, skipped, no-run, digest
  ausente ou identidade stale.
- **Quando** o report é validado.
- **Então** não é sucesso e a validação falha fechado.

### US-1330 — QA failure handoff

Como sistema de qualidade, quero encaminhar falhas e limites de execução sem
permitir que o agente mude o gate ou transforme hipótese em aprovação.

#### AC-1330 — Falha advisory

- **Dado** report bound com resultado `Failed` e artifact digest válido.
- **Quando** o handoff é criado.
- **Então** ele identifica a falha como `Failure`, permanece advisory e não
  pode desativar checks ou autorizar release.
- **Dado** resultado `TimedOut`, `Malformed`, `Stale`, `Skipped` ou `NoRun`.
- **Quando** o report é avaliado.
- **Então** ele não é sucesso nem pode ser usado para liberar o próximo gate.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Observabilidade

O contrato expõe apenas profile/schema/policy revision, project/task/worktree/
branch, SHA/tree, comando tipado, tentativa, duração, status, contagens e
output/artifact digests. Não expõe texto bruto de comandos, logs, prompts ou
segredos.

## Segurança

A execução de comandos e o armazenamento de artefatos pertencem a boundaries
externas. O domínio rejeita shell/arbitrary, paths ou instruções textuais como
capability e não possui APIs para alterar expectations, desabilitar checks ou
autorizar release.

## Rollback

Remover o módulo, seus testes, docs e verificação ONP não altera migrations,
branches, gates ou execução externa. Consumidores futuros devem permanecer
bloqueados até nova versão compatível do contrato.

## Definition of Done

Contrato puro em `agent-core`, testes positivos/negativos rastreáveis,
documentação e verificação ONP passam; nenhum executor, adapter, secret ou
mudança de expectation é introduzido.
