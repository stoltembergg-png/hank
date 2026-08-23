# Tasks: Python worker

> feature: python-worker

## T-672 — Implementar worker mínimo com lifecycle controlado [concluida]

- Refs: US-620, AC-683, AC-684, AC-685
- Arquivos: python/runtime/worker.py, python/runtime/__main__.py, python/runtime/__init__.py
- Notas: loop NDJSON stdin/stdout com estados espelhando `WorkerSession`; handshake validado (versão, worker_id, capabilities) responde `handshake_accepted`; mensagens pré-handshake ou versão inválida encerram com exit 1; argumentos fora da allowlist encerram com exit 2; linhas malformadas/kinds desconhecidos respondem erro bounded e mantêm o canal; shutdown responde ack e encerra com exit 0.

## T-673 — Cobrir integração de processo e não-execução [concluida]

- Refs: US-620, AC-683, AC-684, AC-685, AC-686, AC-687
- Arquivos: crates/agent-protocol/tests/worker_process_contract.rs, .spec/verification/python-worker.json
- Notas: harness de processo com transcript; lifecycle feliz, negações fail-closed (exit 1/2), canal resiliente, request responde `not_supported` sem ecoar payload e a resposta valida como `WorkerMessage`; contrato de fonte sem manifestos de dependência e sem env/exec; sessão in-process sem Python.

## T-674 — Registrar verificação e documentar sidecar [concluida]

- Refs: US-620, AC-683, AC-684, AC-685, AC-686, AC-687
- Arquivos: .github/workflows/onp-sdd-evidence.yml, docs/python-worker.md
- Notas: passo `Verify python worker` no workflow de evidência; documentação de entrypoint, framing, lifecycle, exit codes, isolamento e rollback do sidecar.

## T-679 — Implementar supervisor Rust bounded [concluida]

- Refs: US-622, AC-694, AC-695, AC-696, AC-697, AC-698
- Arquivos: crates/agent-runtime/src/python_lifecycle.rs, crates/agent-runtime/src/lib.rs
- Notas: máquina de estados, spawn/cleanup real, identidade project/session/task/trace, operation keys, budget reservation/release, restart cap/backoff, ambiente limpo e eventos redigidos.

## T-680 — Cobrir lifecycle com testes reais e negativos [concluida]

- Refs: US-622, AC-694, AC-695, AC-696, AC-697, AC-698
- Arquivos: crates/agent-runtime/tests/python_lifecycle_contract.rs
- Notas: processo real, readiness, stop, crash, timeout, cancelamento, restart bounded, dedupe, falha de comando, budget e isolamento de projeto.

## T-681 — Documentar policy do lifecycle [concluida]

- Refs: US-622, AC-694, AC-695, AC-696, AC-697, AC-698
- Arquivos: docs/python-worker-lifecycle.md, docs/python-worker.md
- Notas: state machine, cleanup, policy de restart, identidade, limites e rollback documentados; o passo existente `Verify python worker` no workflow cobre os ACs da feature.

## T-682 — Registrar verificação SDD do lifecycle [concluida]

- Refs: US-622, AC-694, AC-695, AC-696, AC-697, AC-698
- Arquivos: .spec/verification/python-worker.json
- Notas: prova CI no head exato registra exitCode 0 e AC-694..AC-698 PASS; o workflow verifica a feature e exige digest do artifact.

## T-683 — Implementar SDK Python protocol-only [concluida]

- Refs: US-623, AC-699, AC-700, AC-701, AC-702, AC-703
- Arquivos: python/sdk/__init__.py, python/sdk/client.py, python/sdk/errors.py
- Notas: wrapper JSON-RPC com handshake, request/response, health, cancel notification e shutdown; sem subprocesso, tool registry ou persistência. Validado pelo contrato Python e pela evidência ONP de AC-699..AC-703.

## T-684 — Cobrir SDK com contratos determinísticos [concluida]

- Refs: US-623, AC-699, AC-700, AC-701, AC-702, AC-703
- Arquivos: python/tests/test_sdk.py, test/python-sdk-contract.spec.test.js, tools/run-all-tests.mjs
- Notas: streams framed in-memory, validação de contexto/IDs/payload, cancel/shutdown, erros redigidos e comando oficial do agregador; suíte direcionada e agregador passam sem dependência externa.

## T-685 — Documentar SDK e registrar verificação [concluida]

- Refs: US-623, AC-699, AC-700, AC-701, AC-702, AC-703
- Arquivos: docs/python-sdk.md, .spec/verification/python-worker.json
- Notas: API, trust boundary, versionamento, limites, rollback e evidence por SHA documentados em `docs/python-sdk.md`; `python-worker.json` registra PASS para AC-699..AC-703 no head integrado `46cbd62`.

## T-686 — Implementar registration adapter Python declarativo [concluida]

- Refs: US-624, AC-704, AC-705, AC-706, AC-707, AC-708
- Arquivos: crates/tool-core/src/python_registration.rs, crates/tool-core/src/lib.rs
- Notas: implementado e integrado no PR #174 (`7ef342b`); valida schema Python, worker/project/trace e origem project-scoped, criando handler declarativo que sempre nega execução até evaluator futuro.

## T-687 — Cobrir registro, isolamento e rollback [concluida]

- Refs: US-624, AC-704, AC-705, AC-706, AC-707, AC-708
- Arquivos: crates/tool-core/tests/python_registration_contract.rs
- Notas: `python_registration_contract.rs` cobre registro válido, environment/identity/origin inválidos, duplicata, isolamento de projeto e resolução sem execução; AC-704..AC-708 têm PASS no agregado em `46cbd62` e o contrato direcionado passou 4/4 no `2361709`.

## T-688 — Documentar registration e registrar verificação [concluida]

- Refs: US-624, AC-704, AC-705, AC-706, AC-707, AC-708
- Arquivos: docs/python-tool-registration.md, .spec/verification/python-worker.json
- Notas: `docs/python-tool-registration.md` e `python-worker.json` foram entregues no PR #174 (`7ef342b`); trust boundary, evaluator gate, lifecycle, rollback e evidence por SHA estão registrados, com AC-704..AC-708 PASS no head integrado `46cbd62`.

## T-689 — Implementar executor com caminho único [concluida]

- Refs: US-625, AC-709, AC-710, AC-711, AC-712, AC-713
- Arquivos: crates/agent-runtime/src/python_executor.rs, crates/agent-runtime/src/lib.rs, crates/agent-runtime/Cargo.toml
- Notas: pipeline registry→evaluator→lifecycle→JSON-RPC sobre `WorkerTransport`; input/output bounded pelo schema com teto do executor; dedupe de operation key; janela com timeout/cancel; TerminalResult mapeado deterministicamente para ToolOutcome; crash do canal → SandboxError com reaping.

## T-690 — Cobrir contrato com fixture worker sem Python [concluida]

- Refs: US-625, AC-709, AC-710, AC-711, AC-712, AC-713, AC-714
- Arquivos: crates/agent-runtime/tests/python_executor_contract.rs
- Notas: fixture worker em memória roteia o transporte; matrix de negação, timeout/cancel, dedupe sem re-dispatch, payload hostil como dado, identidade divergente e crash com reaping.

## T-691 — Documentar fluxo de execução e registrar evidência [concluida]

- Refs: US-625, AC-709, AC-710, AC-711, AC-712, AC-713, AC-714
- Arquivos: docs/python-executor.md, .spec/verification/python-worker.json
- Notas: fluxo ponta a ponta, trust boundary, limits, negações, rollback/troubleshooting; evidência regenerada por verify.

## T-692 — Implementar manifesto e lock de ambiente [concluida]

- Refs: US-626, AC-715, AC-716, AC-717, AC-718
- Arquivos: crates/agent-runtime/src/python_environment.rs, crates/agent-runtime/src/lib.rs
- Notas: implementado e integrado no commit `31cb9f8`; manifesto versionado, packages hash-pinned, source allowlist, project-local lock, escrita atômica e rollback, sem instalação global.

## T-693 — Cobrir ambiente com contratos determinísticos [concluida]

- Refs: US-626, AC-715, AC-716, AC-717, AC-718
- Arquivos: crates/agent-runtime/tests/python_environment_contract.rs
- Notas: `python_environment_contract.rs` cobre ordenação, hashes, duplicatas, traversal/source inválido, lock, persistência, rollback e isolamento project-scoped; contrato direcionado passou 2/2 no estado atual.

## T-694 — Documentar policy de ambiente e verificar SDD [concluida]

- Refs: US-626, AC-715, AC-716, AC-717, AC-718
- Arquivos: docs/python-environment.md, .spec/verification/python-worker.json
- Notas: `docs/python-environment.md` documenta lifecycle do manifesto, lock, source policy, rollback e não instalação global; `python-worker.json` registra PASS para AC-715..AC-718 no head integrado `46cbd62`.

## T-698 — Implementar matriz Python sobre evaluator comum [em-andamento]

- Refs: US-628, AC-727, AC-728, AC-729, AC-730
- Arquivos: crates/tool-core/src/python_permissions.rs, crates/tool-core/src/lib.rs
- Notas: capabilities FS/network/process/package, project scope, approval, budget e revoke; sem segundo evaluator.

## T-699 — Cobrir policy Python com security matrix [em-andamento]

- Refs: US-628, AC-727, AC-728, AC-729, AC-730
- Arquivos: crates/tool-core/tests/python_permissions_contract.rs
- Notas: allow/deny default, capability ausente, approval ausente, cross-project, revoke e budget.

## T-700 — Documentar permissões e registrar verificação [pendente]

- Refs: US-628, AC-727, AC-728, AC-729, AC-730
- Arquivos: docs/python-permissions.md, .spec/verification/python-worker.json
- Notas: capability matrix, threat boundary, evaluator comum, revocation, rollback e evidence por SHA.

## T-695 — Implementar captura e redação bounded [concluida]

- Refs: US-627, AC-719, AC-720, AC-721, AC-722, AC-723
- Arquivos: crates/agent-runtime/src/python_logs.rs, crates/agent-runtime/src/lib.rs
- Notas: registros com nível/fonte/sequência/correlação; redactor determinístico (secretos mascarados com encadeamento Bearer, ANSI/controle removidos, traversal neutralizado, truncagem bounded); buffer com capacidade, budget de bytes, drop de mais antigos com contador, rotação drena e registro acima do budget não é retido; isolamento por projeto na leitura.

## T-696 — Logger estruturado no worker Python [concluida]

- Refs: US-627, AC-724
- Arquivos: python/runtime/logging.py, python/runtime/worker.py
- Notas: single-line JSON em stderr com sanitize espelhado (redação/controle/truncagem); worker rota suas mensagens de stderr pelo logger; stdout segue exclusivo do transporte.

## T-697 — Cobrir contrato e registrar evidência [concluida]

- Refs: US-627, AC-719, AC-720, AC-721, AC-722, AC-723, AC-724
- Arquivos: crates/agent-runtime/tests/python_logs_contract.rs, .spec/verification/python-worker.json
- Notas: correlação, redação (inclusive cadeia Authorization: Bearer), volume/rotação/budget fail-closed, malformadas definidas, isolamento por projeto e end-to-end com worker real (stderr estruturado bounded sem eco de payload).
