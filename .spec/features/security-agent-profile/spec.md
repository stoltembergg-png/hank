# Spec: security agent profile

> feature: security-agent-profile
> status: auditada

## Contexto

PR-211 define um perfil advisory para organizar threat cases, controles e testes
negativos. O domínio distingue hipótese de evidência, valida identidade exata e
mantém bloqueios quando fixtures ou artifacts não podem ser provados.

## Boundary e não-escopo

- `security-core` contém somente manifest, classificações, validação e handoff
  determinísticos.
- O profile não explora produção, acessa secrets, executa payloads, altera gates,
  muda Rulesets/CODEOWNERS, aprova ou publica.
- Descrições e artifacts externos são dados não confiáveis; não existe campo de
  shell, prompt executável ou instrução privilegiada.
- Logs e artifacts são representados por metadados e digests bounded, nunca por
  conteúdo bruto.

## Histórias

### US-1331 — Threat manifest bounded

Como avaliador de segurança, quero receber casos de ameaça e controles limitados
para que a análise seja reproduzível e não conceda exploração.

#### AC-1331 — Manifest e escopo

- **Dado** profile válido e manifest vinculado a project/task/repository/worktree,
  branch, SHA/tree e policy revision.
- **Quando** os controles são allowlisted e o manifest respeita os limites.
- **Então** a autorização retorna um permit advisory sem acesso a secrets,
  exploração ou mutação de gate.
- **Dado** controle desconhecido, scope diferente ou metadata fora dos limites.
- **Quando** o manifest é autorizado.
- **Então** a operação falha fechado antes de qualquer efeito.

### US-1332 — Security evidence

Como orquestrador, quero vincular findings a TM/control/test IDs e evidence ao
SHA/tree/policy/artifact corretos, distinguindo hipótese de evidência.

#### AC-1332 — Prova e identidade

- **Dado** evidência `Passed` ou `Failed` com digests válidos para cada threat
  case.
- **Quando** o report é validado.
- **Então** ele preserva a classificação e identidade exata.
- **Dado** SHA/tree/policy stale, artifact ausente, fixture missing/skipped/no-run
  ou evidence malformed.
- **Quando** o report é validado.
- **Então** ele não é `Pass` e a validação falha fechado.

### US-1333 — Security escalation

Como sistema de qualidade, quero encaminhar falhas, blockers e hipóteses sem
transformá-los em aprovação ou bypass.

#### AC-1333 — Handoff advisory

- **Dado** finding de evidência `Failed` ou blocker não resolvido.
- **Quando** o handoff é criado.
- **Então** ele preserva IDs, status e digest, mas não pode aprovar, alterar gate,
  acessar secrets ou executar exploração.
- **Dado** artifact hostil ou texto instruction-like.
- **Quando** é incorporado ao manifest.
- **Então** permanece dado não confiável e não cria capability ou autoridade.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Observabilidade

O contrato expõe apenas evaluator/policy revision, project/task/repository/
worktree/branch, SHA/tree, TM/control/test IDs, classificação, severity, status,
duração/bytes e digests. Conteúdo bruto, secrets e payloads não são retornados.

## Segurança

O agente é advisory e provider-neutral. Exploração real, secrets, mutações
externas e execução de testes pertencem a adapters autorizados e permanecem fora
deste slice. Hypothesis, missing, stale, malformed e blockers nunca são
promovidos a `Pass`.

## Rollback

Remover o módulo, testes, docs e verificação ONP remove somente o contrato de
domínio; não altera gates, migrations, branches ou runtime externo.

## Definition of Done

Manifest, evidence e handoff bounded implementados em `security-core`, testes
positivos/negativos rastreáveis, documentação e verify ONP passam sem conceder
autoridade executável.
