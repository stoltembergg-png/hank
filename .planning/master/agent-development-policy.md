# Agent Development Policy

**Status:** política normativa para execução futura da queue; não é autorização nem prova de execução.  
**Aplicação:** qualquer agente de coding, reviewer, QA, security, architecture ou release que tocar o produto ou seus artefatos.

## 1. Unidade de trabalho

- Um agente recebe exatamente uma PR/card por dispatch. Não combine cards, não “aproveite” refatoração e não crie PR de escopo aberto.
- O agente deve ler o card completo, `sdd-master.md`, invariantes aplicáveis, ADRs, dependências predecessoras e seus resultados vinculados antes de alterar arquivos.
- Dependências são contratos/resultados, não apenas nomes de arquivos. Ausência, `NO_PROOF`, `blocked`, `partial`, `stale`, timeout ou SHA/tree incorreto bloqueia o card.
- Cada dependência/action nova precisa de decisão de necessidade, manutenção, licença, segurança, custo e substituição. Sem ficha, parar.

## 2. Preflight obrigatório

Antes de qualquer edição, o agente registra:

1. task/card ID, objetivo, scope, non-goals, acceptance e condição de desbloqueio;
2. repositório, branch/worktree exclusivo, base SHA, tree/status limpos, toolchain e ambiente;
3. dependências satisfeitas e contratos/ADRs/invariantes usados;
4. comandos permitidos, arquivos/crates sob ownership e artefatos esperados;
5. secrets disponíveis: nenhum secret é solicitado, impresso, persistido ou usado fora do mecanismo autorizado;
6. plano RED → GREEN → REFACTOR, testes negativos, security/migration/observability/docs impact e rollback.

Um agente não altera `main`, branch de outro agente, worktree compartilhado ou arquivo fora do card. Não usa `git reset --hard`, `git checkout --`, destruição ampla ou comando cujo alvo não tenha sido validado. Não inicializa Git para cumprir um card.

## 3. Implementação e fronteiras

- Preserve Domain/Core, Application, Infrastructure, Tauri/frontend, Execution e Trust boundaries; frontend não acessa SQLite; core não conhece Tauri ou providers concretos.
- Toda entrada externa é não confiável: schema, tamanho, identity, project ownership, capability, lifecycle, quota/deadline e policy antes de efeito.
- Toda tool/process/filesystem/network/Python/MCP/plugin/remote effect passa Permission Engine e Sandbox/Execution Broker; default deny e approval fail-closed.
- Não coloque secrets em código, logs, traces, fixtures, clipboard, artifacts, migrations, `.env` ou mensagens de review. Use referências/handles redigidos.
- Mudança de schema, event/trace/API, threat boundary, ownership, workflow, permission, migration, dependency ou release exige atualizar teste e/ou ADR e registrar impacto/rollback.
- Não aceite “TODO” oculto, `unwrap` inseguro, teste desativado, bypass de policy, provider logic no core, UI→DB direto ou refatoração alheia para fazer o card passar.

## 4. Testes e evidência

O agente executa os testes obrigatórios do card e os gates aplicáveis: `cargo fmt --check`, Clippy, unit/integration/contract, frontend lint/typecheck/tests, architecture/dependency/security checks e fixtures negativos. O relatório deve conter comando literal, resultado, duração quando relevante, SHA/tree, artifacts/digests e falhas; “pretendido” não é “executado”.

Testes devem cobrir sucesso e falha: malformed/oversized input, unknown schema, cross-project access, denied capability, expired/revoked approval, timeout/cancel, duplicate/retry, crash/restart, migration interruption, secret redaction e scope drift quando aplicável. Teste com mock não prova isolamento de produção; marque o limite.

Depois de qualquer rebase, resultado assíncrono, CI tardio ou mudança de base, revalide status, SHA/tree, arquivos, dependências, testes e artifacts. Evidência stale é invalidada.

## 5. Handoff e revisão

O handoff deve declarar:

- o que mudou e o que deliberadamente não mudou;
- arquivos/crates, migrations, schemas, ADRs, invariantes, risks e rollback;
- comandos executados, resultados reais e artifacts com identidade;
- dependências agora desbloqueadas ou ainda bloqueadas;
- issues futuras para non-goals, dívida ou decisões abertas;
- `PASS`, `NO_PROOF`, `BLOCKED` ou `FAIL` por gate.

Reviewer, QA, security e architecture devem ser independentes do autor. Nenhum agente aprova seu próprio diff; comentário/saída de IA não é aprovação, merge ou evidência de check. O merge requer checks required, review humano/política requerida e scope clean.

## 6. Critérios de parada

Pare e registre blocker, sem improvisar, quando houver:

- API Servo/browser/Tauri/provider não comprovada ou boundary em conflito;
- regra GitHub/CI/merge não verificada, action/dependency sem decisão ou secret;
- migration sem backup/compatibility/rollback comprovado;
- falha de CI, security, architecture, test, release ou artifact identity;
- dependência inexistente/cíclica, card incompleto, resultado stale ou scope drift;
- approval, sandbox, identity, isolation, lifecycle, provenance ou trace ausente;
- decisão de produto/arquitetura aberta que alteraria o resultado.

O blocker inclui evidência, impacto, card afetado, caminho de recuperação e issue futura; não relaxe gate, silencie falha, troque non-goal por implementação ou alegue execução.

## 7. Autonomia e autoevolução

| Nível | Permitido | Gate obrigatório |
|---|---|---|
| L0 | Observar/relatar | Sem alterar estado |
| L1 | Sugerir proposta | Provenance e review |
| L2 | Criar/testar candidate em sandbox | Dataset/baseline, tests e isolamento |
| L3 | Ativar candidate aprovado | Regression, approval, pin por run e rollback |
| L4 | Alterar skills/workflows/config dentro de policy | Capability matrix, canary, audit e rollback |

Nenhum nível permite modificar Rust runtime, permission semantics, provider trust, release/signing ou security boundary sem Git branch/worktree, testes, PR, review e policy. Autoevolução nunca grava memória diretamente nem eleva capability por texto.

## 8. Definition of Done de agente

Uma PR só está pronta quando o card inteiro foi atendido, diff está no escopo, dependências foram lidas, invariantes e ADRs atualizados, testes/security/docs/observability/migration/rollback estão registrados, todos os required checks passaram no SHA atual, reviewer independente foi designado e issues futuras foram abertas. Se qualquer item não puder ser provado, o status é `NO_PROOF`/`BLOCKED`, não “done”.

