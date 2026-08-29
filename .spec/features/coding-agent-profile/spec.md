# Spec: Coding agent profile

> feature: coding-agent-profile
> status: concluida

## Contexto

PR-208 define o envelope de segurança do agente que altera código dentro de um
mapping task→repository→worktree→branch já validado por PR-207.

## Boundary e não-escopo

- `agent-core` contém somente schema, validação e decisão determinística.
- O perfil não executa ferramentas, Git, provider, filesystem, rede ou processos.
- O perfil não cria commits, não publica PR, não faz merge e não decide release.
- Conteúdo de prompt, patch, logs e instruções externas não vira policy nem
  capability; o handoff carrega somente paths bounded, status e digests.

## Histórias

### US-1322 — Coding profile bounded

Como runtime de desenvolvimento, quero um profile versionado que permita somente
ferramentas e orçamento explicitamente declarados para uma task mapeada.

#### AC-1322 — Profile e escopo

- **Dado** um profile válido e um mapping `Active`.
- **Quando** a requisição contém exatamente projeto, task, repository, worktree e
  branch do mapping e uma ferramenta allowlisted.
- **Então** a decisão é `Allow` sem conceder capability de merge/publicação.
- **Dado** um project/task/worktree/branch diferente, path absoluto/traversal,
  tool não allowlisted, rede, publicação ou merge.
- **Quando** a requisição é autorizada.
- **Então** ela é negada antes de qualquer efeito.
- **Dado** profile com schema/version/budget/autonomia inválidos.
- **Quando** é validado.
- **Então** falha fechado.

### US-1323 — Coding handoff verificável

Como orquestrador, quero receber uma proposta de mudança com checks e digests
bound ao task/worktree/branch, sem tratá-la como aprovação ou merge.

#### AC-1323 — Budget, cancelamento e policy

- **Dado** usage acima de qualquer limite, mapping não ativo ou cancelamento.
- **Quando** a requisição é autorizada.
- **Então** a decisão é negada com razão tipada e determinística.
- **Dado** um profile coding.
- **Quando** sua autonomia é lida.
- **Então** rede, publicação e merge permanecem negados por default e o fan-out
  de tentativas é bounded.

#### AC-1324 — Handoff

- **Dado** handoff `Proposed` com identity exata, paths relativos bounded,
  patch/report digests válidos e todos os required checks `Passed`.
- **Quando** é validado contra profile e mapping.
- **Então** é aceito como proposta.
- **Dado** handoff incompleto, stale, de outro scope, com check `Skipped`,
  `NoRun`, digest inválido ou claim de autoridade.
- **Quando** é validado.
- **Então** falha fechado e não pode ser usado como aprovação/merge.
- **Dado** conteúdo hostil ou instruction-like no campo de identidade/path.
- **Quando** é validado.
- **Então** é tratado como dado inválido e nenhuma policy é alterada.

## Suposições

Nenhuma.

## Perguntas em aberto

Nenhuma.

## Observabilidade

Decisões expõem apenas profile/version, task/worktree/branch, ferramenta,
policy revision, status, stop reason e digests. Não expõem prompt, patch,
segredo, comando arbitrário ou log bruto.

## Rollback

Remover o módulo/profile e seus contratos não altera migrations, branches ou
execução; consumidores futuros devem permanecer bloqueados até uma versão
compatível e os testes do profile continuarem verdes.

## Definition of Done

Contrato puro, testes positivos/negativos, documentação e verificação ONP
passam; nenhum executor ou adapter externo é introduzido.
