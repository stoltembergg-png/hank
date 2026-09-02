# Executable Review Remediation Queue — PR-416

**Status:** `IN_PROGRESS`. This card is the next increment after merged PR-415 and is
implemented only after predecessor PR-397 is verified as merged.

### PR-416 — Bounded external reviewer remediation workflow

- **ID:** PR-416
- **Categoria:** CI / SECURITY / TESTING
- **Milestone:** M24 — REVIEW REMEDIATION AUTOMATION
- **Título:** Bounded external reviewer remediation workflow
- **Objetivo:** Transformar uma sugestão concreta do Aikido ou CodeRabbit em um patch
  validado pelo Xiaomi MiMo v2.5 e em uma draft PR isolada para revisão humana.
- **Problema resolvido:** O resolvedor anterior encerrava conversas sem avaliar a
  viabilidade nem produzir evidência verificável da correção.
- **Escopo:** Coleta de finding, julgamento de viabilidade por patch limitado, guards
  de segurança, validação no SHA exato, evidência e publicação de draft PR.
- **Não-escopo:** Resolver ou ocultar conversas, modificar a PR de origem, aprovar,
  fazer merge, rebase, publicar release ou alterar regras/proteções.
- **Dependências anteriores:** PR-397 mergeado; contratos de reviewer e fix-review
  existentes.
- **Requisitos funcionais:** Aceita apenas Aikido/CodeRabbit vinculados a uma PR do
  mesmo repositório; resposta sem patch seguro vira `HUMAN_REQUIRED`; patch seguro
  passa por validação e gera no máximo uma draft PR por fingerprint.
- **NFRs:** Endpoint/modelo fixos; segredo isolado no job de proposta; sem código da
  PR não confiável executado com credencial; limites de tamanho; fail-closed;
  idempotência; logs e artefatos redigidos.
- **Critérios de aceite verificáveis:** Foreign/fork/stale/duplicate/oversized/
  forbidden-path/prompt-injection findings não chegam à publicação; somente patch
  aplicável e `git diff --check` aprovado geram draft PR. Scripts de build, teste ou
  pacote da PR de origem não são executados pelo workflow de remediação.
- **Testes unitários:** Contratos, redaction, prompt/client, patch guards e API.
- **Testes de integração:** Coleta→proposta→validação→descriptor de publicação com
  APIs falsas, sem chamada real ao Xiaomi.
- **Testes negativos:** Secret leakage, auto-merge/approval, workflow mutation,
  branch mutation, stale SHA, cycle cap, binary/symlink/submodule e path traversal.
- **E2E obrigatório quando aplicável:** Workflow contract + fixture offline; checks
  completos continuam autoridade na draft PR.
- **Verificações de segurança:** `XIAOMI_MIMO_API_KEY` somente no job `propose`;
  write permission somente no job `publish`; actions fixadas por SHA.
- **Observabilidade:** Fingerprint, source SHA, patch digest, tree digest, status
  `NOOP`/`HUMAN_REQUIRED`/`PROPOSED`/`VALIDATED`/`PUBLISHED` e rollback documentado.
- **Evidência:** SHA/tree/policy/schema/fixture/environment/artifact digest e estado
  PASS/FAIL/BLOCKED/NO_PROOF.
- **Documentação:** Guia operacional, rotação do segredo e rollback.
- **Rollback:** Desabilitar/reverter workflow, revogar/rotacionar segredo, fechar a
  draft PR indesejada e remover somente a branch de automação.
- **Definition of Done:** Testes focados, Actionlint, Quality Integrity e gates
  aplicáveis passam; publicação confirma digest/tree, identidade live da PR, lista de
  arquivos staged e worktree limpo; draft PR fica sem auto-merge e conversa de origem
  não é encerrada automaticamente. Os checks normais da draft continuam autoridade
  para validar o código.
- **Condição para desbloquear a próxima PR:** Nenhuma etapa posterior é desbloqueada
  automaticamente; a mudança precisa de revisão/merge humano e checks obrigatórios.
