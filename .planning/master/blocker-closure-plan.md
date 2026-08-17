# Plano de fechamento dos BLOCKERs

**Escopo:** fechamento documental e de evidência dos 27 achados `BLOCKER` do review atual. Este arquivo não implementa produto, não altera as fontes e não converte planejamento em prova.

**Fontes lidas:** `.planning/reviews/specification-review.md`, `.planning/master/sdd-master.md`, `.planning/master/architecture-invariants.md`, `.planning/master/dependency-dag.md`, os três arquivos `.planning/queue/*.md` e `.planning/reviews/architecture-boundaries.md`.

**Regra de status:** `PARTIAL/NO_PROOF` significa que o SDD mestre/fonte consolidada já contém uma decisão ou contrato parcial, mas não há evidência vinculada ao SHA/tree/policy. `OPEN/NO_PROOF` significa que o artefato normativo ainda não está definido de modo suficiente. Nenhum item é `RESOLVED`. Não foram executados testes, runtime, migrations, CI, sandbox, assinatura ou release nesta entrega.

## Contagem e inventário

Há 27 IDs únicos, cada um descrito uma vez como heading de blocker:

`ARCH-001`, `ARCH-002`, `SEC-001`, `SEC-002`, `SEC-003`, `SEC-004`, `DATA-001`, `DATA-002`, `DATA-003`, `RUNTIME-001`, `MA-001`, `WF-001`, `API-001`, `OBS-001`, `TEST-002`, `DIST-001`, `DIST-002`, `EXT-001`, `EXT-002`, `EXT-003`, `EXT-004`, `EVOL-001`, `EVOL-002`, `GOV-001`, `GOV-002`, `GOV-003`, `ROAD-001`.

## Waves, caminho crítico e cross-cutting

As waves abaixo são waves de fechamento de contrato/evidência; não autorizam implementação de um card sem o DAG, o predecessor e o review independente.

| Wave | Objetivo | Blockers | Gate de saída |
|---|---|---|---|
| W0 | Fundação, fronteiras e execução por agentes | ARCH-001, ARCH-002, GOV-001, GOV-002, GOV-003 | ADRs de arquitetura/governança, matriz de ownership, queue validator e PR Execution Contract aceitos; fixtures negativas definidas |
| W1 | Identidade, capability, sandbox, secrets, API e schema base | SEC-001..004, RUNTIME-001, API-001, DATA-001 | AB-002/003/004/006/014, schema de dados e precedência de instruções com contratos e casos negativos |
| W2 | Durabilidade, delegação, workflows e observabilidade | DATA-002, DATA-003, WF-001, MA-001, OBS-001 | state machines, outbox/recovery, migration matrix, envelope de delegação e trace-redaction matrix vinculados |
| W3 | Python, MCP, plugins, remoto e evolução controlada | EXT-001..004, EVOL-001..002 | AB-007/009/010/011/017/020 e threat fixtures com capability narrowing, quarantine, revoke e rollback |
| W4 | Testes adversariais, distribuição e release | TEST-002, DIST-001..002, ROAD-001 | threat regression manifest, artifacts assinados, updater/rollback matrix e release contract com decisão `NO_GO`/elegível para aprovação humana |

### Caminho crítico

O caminho crítico normativo é W0 → W1 → W2 → W3 → W4. Ele acompanha o caminho máximo do DAG, sem declarar execução: `PR-001 → PR-002 → PR-003 → PR-004 → PR-008 → PR-011 → PR-013 → PR-014 → PR-015 → PR-016 → PR-017 → PR-018 → PR-019 → PR-020 → PR-021 → PR-022 → PR-056 → PR-057 → PR-058 → PR-060 → PR-061 → PR-062 → PR-067 → PR-068 → PR-069 → PR-070 → PR-071 → PR-072 → PR-073 → PR-076 → PR-077 → PR-078 → PR-079 → PR-081 → PR-082 → PR-083 → PR-084 → PR-085 → PR-086 → PR-087 → PR-088 → PR-089 → PR-090 → PR-109 → PR-110 → PR-111 → PR-133 → PR-134 → PR-155 → PR-156 → PR-158 → PR-160 → PR-161 → PR-162 → PR-163 → PR-165 → PR-166 → PR-167 → PR-169 → PR-170 → PR-171 → PR-172 → PR-173 → PR-174 → PR-175 → PR-176 → PR-177 → PR-181 → PR-182 → PR-186 → PR-187 → PR-188 → PR-189 → PR-190 → PR-191 → PR-192 → PR-195 → PR-252 → PR-253 → PR-254 → PR-255 → PR-256 → PR-259 → PR-260 → PR-261 → PR-262 → PR-265 → PR-266 → PR-267 → PR-268 → PR-269 → PR-270`.

O caminho pode abrir lanes apenas nos pontos indicados no DAG. Cross-cutting blockers que devem bloquear todas as lanes downstream são: ARCH-002 (ownership/arestas), SEC-001 e SEC-002 (identity/capability), SEC-003 e SEC-004 (sandbox/secrets), API-001 (envelopes), DATA-001..003 (estado/migration/recovery), RUNTIME-001 (instruções), OBS-001 (trace/redaction), TEST-002 (threat regression), GOV-003 (execução e review) e ROAD-001 (release/no-ship). EXT-002..004 propagam o mesmo bloqueio às lanes de extensões e remoto; DIST-001..002 propagam-no ao artefato final.

## W0 — Fundação, fronteiras e execução por agentes

### ARCH-001

- **Seção/linhas:** `specification-review.md:23-33`; source: “Produto e objetivo”, “Arquitetura normativa proposta” e “Regras imutáveis”, item 1.
- **Evidência:** a fonte chama “Rust + Tauri 2” de núcleo e também afirma que Tauri não é Agent Core; não define shell, adapter, processo ou API host.
- **Risco:** regras, estado ou runtime podem cair no processo/UI Tauri, quebrando adapters não-Tauri e alterando a trust boundary por superfície.
- **Decisão normativa exigida:** `agent-core`/runtime é independente; Tauri é somente shell/adaptador de bridge, eventos, janela, deep link e packaging; dependências de domínio para Tauri são proibidas.
- **Artefato owner:** `ADR-AB-001` + manifesto/diagrama de camadas `AB-001` + fixture de import proibido + fixture de adapter não-Tauri.
- **Owner de revisão:** Architecture reviewer independente; Security reviewer para a trust boundary.
- **PRs/milestones afetados:** M0; PR 001–003, 018–019; toda a cadeia M1–M15.
- **Acceptance criteria positivos:** ADR e grafo nomeiam processos, camadas, contratos e edges; `agent-core` compila sem Tauri; adapter fake/CLI exerce o mesmo caso de uso.
- **Acceptance criteria negativos:** import de Tauri/UI em core, regra de domínio no shell ou operação disponível apenas via Tauri deve falhar no validator.
- **Evidência necessária:** diff do ADR/manifesto, grafo Cargo, saída do forbidden-import/adapter fixture, SHA/tree e policy revision; resultado deve ser revisado independentemente.
- **Condição de desbloqueio:** `AB-001` aceito, fixture negativa definida e resultado `PASS` no SHA atual; sem isso PR-001–003 e dependentes continuam bloqueadas.
- **Dependências:** SDD D-001/D-002; architecture-boundaries §§1, 3.1, 3.4 e 5.1; AI-001, AI-003, AI-035; PR-001 antes de PR-002/003.
- **Status atual:** `PARTIAL/NO_PROOF` — o graph validator e os 19 testes negativos passam na `main` atual, mas não há compile do core Rust, adapter fake/CLI executável ou prova de que a camada `frontend` está modelada no grafo.

### ARCH-002

- **Seção/linhas:** `specification-review.md:34-44`; source: “Arquitetura normativa proposta” e inventário de `apps/desktop`, `apps/cli` e crates.
- **Evidência:** há nomes de crates/adapters, mas faltam ownership, dependências permitidas, processo, lifecycle, threading/async e contratos entre Application API, Runtime, storage, Event Bus e adapters.
- **Risco:** ciclos, vazamento de provider/storage no core e autorização divergente; agentes podem implementar contratos incompatíveis.
- **Decisão normativa exigida:** matriz única `camada → responsabilidade → owner → dependências permitidas → dados conhecidos → erros emitidos → processo/lifecycle/compatibilidade`.
- **Artefato owner:** `ADR-AB-001` + matriz de camadas/ownership `AB-001` + architecture graph validator + forbidden-edge/cycle fixtures.
- **Owner de revisão:** Architecture reviewer com reviewer de Runtime/Infrastructure.
- **PRs/milestones afetados:** PR 001, 018–019, 021–024, 056–060, 096–099; todas as milestones.
- **Acceptance criteria positivos:** todos os crates propostos aparecem na matriz; cada comando da API tem owner único; edges core↔provider, core↔Tauri, core↔browser-core e UI↔SQLite têm regra verificável.
- **Acceptance criteria negativos:** crate sem owner, edge concreta substituindo port, ciclo ou comando com dois owners deve produzir `BLOCKED`/`NO_PROOF`.
- **Evidência necessária:** matriz versionada, metadata/grafo, lint de edges proibidas, fixture de ciclo e decisão de compatibilidade, todos ligados ao SHA/tree/policy.
- **Condição de desbloqueio:** matriz completa e aceita, validator reproduzível e review independente; ausência de qualquer crate/edge mantém downstream bloqueado.
- **Dependências:** ARCH-001; SDD D-002/D-004; architecture-boundaries §§3–4; AI-001–006; PR-001, 018–019 e contratos AB-001/002.
- **Status atual:** `PARTIAL/NO_PROOF` — graph validator, ciclo/edge fixtures e 19 testes passam na `main` atual; ownership/lifecycle ainda não cobrem a camada `frontend`, uma port/DTO tipada da Application e execução real dos adapters.

### GOV-001

- **Seção/linhas:** `specification-review.md:771-781`; source: inventário original de PRs e entregável esperado.
- **Evidência:** títulos numerados não bastavam para objetivo, escopo/out, dependências, arquivos, acceptance, testes, security, migration, docs, rollback, riscos, owner e artifact.
- **Risco:** não é possível selecionar exatamente um card executável nem impedir scope drift ou contrato inventado pelo agente.
- **Decisão normativa exigida:** ficha canônica obrigatória para cada card, com os 19 campos, schema validável e distinção entre alvo provável e arquivo existente.
- **Artefato owner:** `AB-038`/AI-038 queue schema validator + matriz de campos/owners + fixture de card incompleto/label mismatch.
- **Owner de revisão:** Planning/Governance reviewer independente; Architecture reviewer para dependências.
- **PRs/milestones afetados:** PR 001–251; todo o contrato de execução por agentes.
- **Acceptance criteria positivos:** os 270 cards são únicos/sequenciais; cada card valida os campos canônicos, dependências, testes, segurança, docs, DoD e desbloqueio.
- **Acceptance criteria negativos:** campo ausente, dependência inexistente/cíclica, label não canônico não normalizado ou `Arquivos prováveis` tratado como existência deve falhar o validator.
- **Evidência necessária:** relatório do parser/schema validator, inventário de 270 cards, lista das 35 divergências de labels e decisão registrada para cada uma, bound ao SHA/tree.
- **Condição de desbloqueio:** parser aceita somente cards completos e a fila corrigida/normalizada é revalidada sem alterar silenciosamente as fontes.
- **Dependências:** AI-038; SDD D-010 e §8; dependency-dag §1; queue contract; GOV-003 para execução.
- **Status atual:** `PARTIAL/NO_PROOF` — queue validator e workflow protegido confirmam 270 IDs/campos no snapshot atual, mas a fonte Markdown ainda não tem snapshot canônico estruturado, a equivalência parser→schema não é completa e os 35 label mismatches permanecem registrados como `NO_PROOF`.

### GOV-002

- **Seção/linhas:** `specification-review.md:782-792`; source: inventário original e Milestone 16.
- **Evidência:** M16 era não numerada na fonte, embora a entrega exigisse queue executável, DAG e PR #001 exata; não havia sequência inicial decidida no material original.
- **Risco:** hardening fica sem owner/dependência e a execução começa em PR arbitrária; “PR #001 exata” não é auditável.
- **Decisão normativa exigida:** decompor M16 em cards numerados, publicar sua ficha canônica e declarar explicitamente o card #001, seus non-goals e acceptance.
- **Artefato owner:** índice M16 `PR-252..PR-270` + `AB-038` queue/DAG validator + ficha canônica de `PR-001` + fixture de IDs duplicados/lacunas.
- **Owner de revisão:** Planning/Governance reviewer independente com Architecture reviewer.
- **PRs/milestones afetados:** M0, M16; releases v0.1–v1.0.
- **Acceptance criteria positivos:** M16 aparece como PR-252..PR-270 sem lacunas/duplicatas; PR-001 é única, apontada por nome/arquivo e tem gates válidos; DAG cobre todos os cards.
- **Acceptance criteria negativos:** card M16 sem número/owner, PR-001 ambígua, predecessor inexistente ou dependência inventada por texto não estrutural produz `BLOCKED`.
- **Evidência necessária:** índice M16, parser de IDs, DAG normalizado, relatório de dependências e decisão de sequência, todos no mesmo SHA/tree/policy.
- **Condição de desbloqueio:** validação mecânica 270/270, card #001 e decomposição M16 aceitos, com divergências da fila explicitamente reconciliadas.
- **Dependências:** GOV-001; SDD D-010 e §8; dependency-dag §§1–4; queue-173-270 §§contrato/gates.
- **Status atual:** `PARTIAL/NO_PROOF` — o parser confirma 270 IDs, PR-001 e M16 no índice e o workflow protegido passou; a exatidão ainda depende do snapshot canônico, da reconciliação de labels/dependências e do gate de autoridade independente.

### GOV-003

- **Seção/linhas:** `specification-review.md:793-803`; source: “Desenvolvimento Agents” (M12) e regra imutável 15.
- **Evidência:** há papéis de agentes e regra Git/testes, mas não há contrato operacional para branch/worktree, base SHA, dirty state, ownership, comandos, secrets, review independente e concorrência.
- **Risco:** agente altera `main`, sobrescreve trabalho paralelo, executa comando fora do escopo ou aprova a própria mudança sem evidência.
- **Decisão normativa exigida:** PR Execution Contract com preflight, branch/worktree, base SHA, scope/non-goals, comandos autorizados, artifacts, gates, handoff, review, rollback e stop/blocker.
- **Artefato owner:** `PR-EXECUTION-CONTRACT.md` + AI-031..034 + fixture de path allowlist/stale SHA/self-approval + run evidence manifest.
- **Owner de revisão:** Governance/Release reviewer independente; Security reviewer para secrets e approval.
- **PRs/milestones afetados:** M0, M12–M13, M16; PR 017, 204–231 e toda a queue.
- **Acceptance criteria positivos:** run registra branch, base SHA, status, arquivos, comandos/resultados, reviewer distinto e artifacts; rebase invalida evidência.
- **Acceptance criteria negativos:** branch `main`, arquivo fora do card, secret no ambiente/artifact, comentário de IA como approval ou reviewer igual ao autor deve falhar o gate.
- **Evidência necessária:** contrato versionado, preflight/scope manifest, log redigido de run, identidade de reviewer e decisão de rollback, ligados ao SHA/tree/policy.
- **Condição de desbloqueio:** contrato aceito e fixture negativa reproduzível; cada card deve apontar o contrato e registrar a aplicação no SHA atual.
- **Dependências:** GOV-001/002; SDD D-010, §2 e §7; AI-031–034; dependency-dag §5; PR-204–217.
- **Status atual:** `PARTIAL/NO_PROOF` — schema, preflight, graph/queue validators e 19 testes passam na `main`; GitHub exige o check protegido atual, mas ainda não existe runner real de agentes que faça deny-before-write, clean-room, secret scan, manifest/digest e reviewer autenticado distinto, nem lifecycle com lease/idempotência/fencing.

## W1 — Identidade, capability, sandbox, secrets, API e schema

### SEC-001

- **Seção/linhas:** `specification-review.md:80-90`; source: “Domínio e políticas” e “Segurança e execução”.
- **Evidência:** isolamento por projeto e delegação são exigidos, mas principal, subject/resource/action, tenancy, sessão de autorização e propagação de contexto não têm definição completa.
- **Risco:** acesso entre projetos e confused deputy; UI, CLI, workflow, MCP e remoto podem aplicar regras diferentes.
- **Decisão normativa exigida:** definir principal/subject/resource/action, project binding, capability token, origem, delegação, narrowing, revogação e contexto verificável em toda operação.
- **Artefato owner:** `ADR-AB-003` + contrato `AB-003` + authorization matrix por entidade/superfície + cross-project negative fixture.
- **Owner de revisão:** Security/Identity reviewer independente.
- **PRs/milestones afetados:** M1–M15; PR 027–035, 044, 068–071, 096–111, 155–172, 232–251.
- **Acceptance criteria positivos:** cada command/tool/event/lookup carrega actor e project scope; grant explícito é mínimo, expirável, auditável e revogável.
- **Acceptance criteria negativos:** projeto A consultando arquivo, memória, secret, tool ou artifact de B, ou delegação que amplia capability, deve falhar sem revelar existência indevida.
- **Evidência necessária:** matriz actor/resource/action, schemas, grant/revoke fixtures, queries scoped e authorization decision logs redigidos, com SHA/tree/policy.
- **Condição de desbloqueio:** AB-003 aceito e negative matrix cobrindo todas as superfícies; qualquer path sem contexto mantém a lane bloqueada.
- **Dependências:** SDD D-003/D-004; architecture-boundaries §§2.1, 5.1, 5.2, 7; AI-006/008/009/021; ARCH-002.
- **Status atual:** `PARTIAL/NO_PROOF` — project isolation, envelope e grants aparecem no SDD/boundaries; não há matriz completa nem prova de negativa cross-project.

### SEC-002

- **Seção/linhas:** `specification-review.md:91-101`; source: “Segurança e execução” e regras imutáveis 5–6.
- **Evidência:** existem nomes de políticas e níveis de confiança, mas faltam schema de capability, precedência, TTL, chamadas compostas, não-interatividade e comportamento em falha.
- **Risco:** ferramenta/workflow/plugin/agente delegado herda poder excessivo; falha de policy pode virar allow.
- **Decisão normativa exigida:** capability por ação/recurso/projeto/principal, default deny/fail-closed, TTL/revoke, fingerprint exato, narrowing, modo não-interativo e auditoria.
- **Artefato owner:** `ADR-AB-006` + contrato `AB-004` + capability/approval matrix + deny/timeout/revoke/replay/parallel fixtures.
- **Owner de revisão:** Security/Permission reviewer independente.
- **PRs/milestones afetados:** M2, M5–M6, M9–M15; PR 044, 096–111, 121, 161, 236, 241, 248.
- **Acceptance criteria positivos:** cada tool call produz decisão explicável e fingerprintada; aprovação só vale para target/args/schema/project/actor/expiry exatos.
- **Acceptance criteria negativos:** deny, timeout do aprovador, revoke, UI ausente, retry alterado, execução paralela ou policy engine indisponível nunca executam o efeito.
- **Evidência necessária:** schema/decision record, matrix de capabilities, fixtures negativas e audit entries sem secret, vinculados a SHA/tree/policy.
- **Condição de desbloqueio:** AB-004 aceito, default deny comprovado e todas as lanes passam pelo mesmo Permission Engine.
- **Dependências:** SEC-001; SDD D-005; architecture-boundaries §6; AI-010/011/034; PR-044/099/236.
- **Status atual:** `PARTIAL/NO_PROOF` — fluxo PermissionRequest, default deny e fingerprint estão descritos; schema completo, chamadas compostas e evidência ainda faltam.

### SEC-003

- **Seção/linhas:** `specification-review.md:102-112`; source: “Segurança e execução”.
- **Evidência:** profiles `trusted/restricted/isolated` e Docker/Podman/SSH/remote/WASM são citados sem roots, symlink, rede, filhos, ambiente, quotas, usuário, mecanismo OS ou fallback.
- **Risco:** `restricted` pode ser apenas promessa; terminal, Python, MCP e plugin podem escapar do projeto/host.
- **Decisão normativa exigida:** profiles por OS e ferramenta, capabilities, canonicalização, quotas, telemetria, falhas e política de indisponibilidade; nunca fallback shell irrestrito.
- **Artefato owner:** `ADR-AB-007` + contrato `AB-006` + sandbox OS matrix + escape/path/symlink/network/process/resource fixtures.
- **Owner de revisão:** Security/Sandbox reviewer independente por OS.
- **PRs/milestones afetados:** M5–M6, M14–M15, M16; PR 103–105, 118–121, 233–243, 247–250.
- **Acceptance criteria positivos:** cada profile declara roots, rede, processos, env, CPU/memória/disco, usuário, timeout e cleanup; execução isolada retorna handle e output bound.
- **Acceptance criteria negativos:** path traversal, symlink escape, rede não allowlisted, processo filho fora do profile, quota bypass ou sandbox ausente não podem executar silenciosamente.
- **Evidência necessária:** matriz por Windows/Linux/macOS, manifests, adversarial fixtures e claim matrix por versão/OS, com artifact/SHA/policy.
- **Condição de desbloqueio:** profile obrigatório selecionado e fixture de indisponibilidade retorna deny ou decisão explícita; nenhum claim de isolamento sem prova por OS.
- **Dependências:** SEC-001/002; SDD D-005; architecture-boundaries §3.15 e §10; AI-012/013/037; PR-103/118/233.
- **Status atual:** `PARTIAL/NO_PROOF` — profiles, broker e fail-closed aparecem no SDD/boundaries; parâmetros por OS e resultados adversariais não foram comprovados.

### SEC-004

- **Seção/linhas:** `specification-review.md:113-123`; source: “Segurança e execução” e “Persistência e distribuição”.
- **Evidência:** keychain/Stronghold são indicados sem lifecycle de credenciais, rotação/revoke, escopo, export/backup/restore, crash dump, clipboard, memória/cache/artifacts ou perda do keychain.
- **Risco:** tokens em logs/traces/backups/artifacts; impossibilidade de revogar/recuperar; migration pode reintroduzir plaintext.
- **Decisão normativa exigida:** secret envelope e opaque handle, storage por plataforma, scope, rotation/revoke, redaction central, encrypted backup/export/import e fail-closed/re-auth.
- **Artefato owner:** `ADR-AB-015` + contrato `AB-014` + secret lifecycle/redaction matrix + golden secret-scan/crash/backup/clipboard fixtures.
- **Owner de revisão:** Security/Secrets reviewer independente; Release reviewer para backup/migration.
- **PRs/milestones afetados:** M3, M4, M6, M12, M14–M16; PR 068–071, 095, 120, 215–217, 250.
- **Acceptance criteria positivos:** adapters recebem apenas handles; rotation/revoke invalida dependências; restore preserva referências cifradas e perda do keychain exige reauth.
- **Acceptance criteria negativos:** valor secreto em SQLite, `.env`, frontend, logs, traces, crash bundle, clipboard, cache, artifact ou backup não criptografado deve falhar o gate.
- **Evidência necessária:** envelope/schema, ACL por OS, redaction golden, secret scan, keychain-loss/rotation/revoke/backup fixtures e artifact identity.
- **Condição de desbloqueio:** AB-014 aceito, fallback plaintext proibido e scans/fixtures vinculados ao SHA/tree/policy sem valores reais.
- **Dependências:** SEC-001/002/003; SDD §5–§6; architecture-boundaries §3.16, §8–§10; AI-014/015/027; PR-068/069/120/250.
- **Status atual:** `PARTIAL/NO_PROOF` — opaque handles, redaction e OAuth safety estão normatizados; lifecycle completo e scans não têm evidência.

### RUNTIME-001

- **Seção/linhas:** `specification-review.md:227-237`; source: hierarquia de instruções em “Domínio e políticas”.
- **Evidência:** a ordem `system → security → project → agent → workflow → skill → conversation → user` não diz se é montagem, precedência de override ou conflito; não torna security não sobrescrevível.
- **Risco:** prompt injection em user/conversation/skill/arquivo/memória remove policy; agentes divergem.
- **Decisão normativa exigida:** merge/override por campo, camadas não sobrescrevíveis, origem confiável, dados não confiáveis delimitados e validação antes do prompt.
- **Artefato owner:** `ADR-AB-011` + `AB-010`/AI-020 + instruction-precedence matrix + prompt-injection persistence fixture.
- **Owner de revisão:** Agent/Runtime reviewer e Security reviewer independentes.
- **PRs/milestones afetados:** M2, M4, M7–M9, M13; PR 043, 082–084, 131, 142, 159–169, 220–228.
- **Acceptance criteria positivos:** tabela determina precedência por campo e preserva security/project/capability; montagem produz digest e origem de cada camada.
- **Acceptance criteria negativos:** texto de usuário, provider, skill, memória ou arquivo não pode rebaixar security policy, capability, isolation ou approval.
- **Evidência necessária:** matrix versionada, deterministic assembly fixtures, injection corpus, output digest e policy/schema revision.
- **Condição de desbloqueio:** `AB-010`/AI-020 aceitos e corpus negativo falha fechado em todos os adapters de contexto.
- **Dependências:** SEC-001/002; SDD §4; architecture-boundaries §§2.1, 3.1, 3.11 e 5.2; PR-043/082/142.
- **Status atual:** `PARTIAL/NO_PROOF` — hierarchy e regra “conteúdo não cria policy” existem; merge por campo e fixture de injection não estão fechados.

### API-001

- **Seção/linhas:** `specification-review.md:385-395`; source: “UI e API”.
- **Evidência:** APIs aparecem só como nomes (`projects.create`, `sessions.send` etc.), sem schemas, errors, auth, versioning, idempotency, pagination, streaming, cancellation ou capability.
- **Risco:** UI/CLI/TUI/remoto criam contratos divergentes, bypassam auth ou perdem streams.
- **Decisão normativa exigida:** Application API tipada/versionada com Command/Result/Event envelopes, stable errors, correlation/idempotency, auth context, capability checks e lifecycle de long-running operations.
- **Artefato owner:** `ADR-AB-002` + contrato `AB-002` + command/result/event schema registry + malformed/oversized/duplicate/stale IPC fixtures.
- **Owner de revisão:** API/Application reviewer independente; Security reviewer para auth/capability.
- **PRs/milestones afetados:** M1–M5, M9–M15; PR 029–031, 088–090, 155–172, 244–251.
- **Acceptance criteria positivos:** cada método tem schema/error/version/idempotency/stream/cancel contract; UI e CLI usam a mesma Application API.
- **Acceptance criteria negativos:** request inválido, unknown version, caller não autorizado, duplicate sem idempotency ou event stale deve falhar antes de side effect.
- **Evidência necessária:** schemas gerados/versionados, contract tests, bridge/CLI fixtures, event sequence e auth decision, bound ao SHA/tree/policy.
- **Condição de desbloqueio:** AB-002 aceito e cada método downstream aponta para seu schema e fixture; nenhum adapter pode inventar envelope.
- **Dependências:** ARCH-001/002 e SEC-001/002; SDD §3.2; architecture-boundaries §§2.1–2.3, 3.2, 3.4–3.5; PR-023/029/089.
- **Status atual:** `PARTIAL/NO_PROOF` — envelopes, estados terminais, idempotency e bridge owner estão descritos; catálogo completo e contract evidence faltam.

### DATA-001

- **Seção/linhas:** `specification-review.md:159-169`; source: “Persistência e distribuição”.
- **Evidência:** entidades/tabelas são enumeradas sem IDs, keys, constraints, optimistic version, timestamps, soft delete, ordering, valid states, tenancy ou retention.
- **Risco:** implementações incompatíveis e perda silenciosa de sessions/runs/usage/tool calls/skills.
- **Decisão normativa exigida:** modelo lógico por entidade, invariants, lifecycle, cardinality, indexes, UTC, global IDs, concurrency, deletion/archive e retention.
- **Artefato owner:** `ADR-AB-004` + contrato de schema `AB-016`/AI-026 + ownership/retention matrix + clean/constraint/repository fixtures.
- **Owner de revisão:** Data/Infrastructure reviewer independente; Security reviewer para tenancy/retention.
- **PRs/milestones afetados:** M1, M4, M7–M11, M13; PR 021–035, 078–081, 122–135, 173–203, 218–231.
- **Acceptance criteria positivos:** migrations do zero reproduzem o modelo; repositories exigem scope e expõem apenas transições válidas; constraints/lifecycle/retention são testáveis.
- **Acceptance criteria negativos:** duplicate ID, cross-project query, estado inválido, timestamp não-UTC, delete fora da policy ou repository que devolve path/segredo devem falhar.
- **Evidência necessária:** schema/migration manifest, ER/ownership matrix, constraints fixtures, repository contract results e schema revision/SHA.
- **Condição de desbloqueio:** modelo lógico aprovado e migration clean/constraint contract ligado a cada entidade antes de PR de persistência.
- **Dependências:** SEC-001; SDD §6; architecture-boundaries §§7–8; AI-008/026/027; PR-021/025/026/027.
- **Status atual:** `PARTIAL/NO_PROOF` — ownership, entidades mínimas e requisitos de migration aparecem; schema normativo e testes de constraints não têm prova.

## W2 — Durabilidade, delegação, workflows e observabilidade

### DATA-002

- **Seção/linhas:** `specification-review.md:170-180`; source: “Workflow, scheduler e eventos” e “Persistência e distribuição”.
- **Evidência:** restart/recovery e Event Bus são exigidos sem transação, outbox/inbox, ordering, delivery, dedupe, idempotency, lease ou reconciliation.
- **Risco:** tool/workflow duplica, evento se perde após commit, run fica preso ou custo duplica no restart.
- **Decisão normativa exigida:** state machine durável, atomicidade change+event, execution IDs, idempotency keys, outbox/inbox, leases/fencing, retry/compensation e recovery por node/tool.
- **Artefato owner:** `AB-012`/AI-023 + `ADR-AB-004` e `ADR-AB-013` + outbox/inbox/recovery matrix + crash-before/after-boundary fixture.
- **Owner de revisão:** Data/Recovery reviewer independente; Runtime reviewer.
- **PRs/milestones afetados:** M1, M4, M9–M11; PR 023–026, 084–090, 176–188, 195–200.
- **Acceptance criteria positivos:** commit de estado e evento é atômico; restart reconcilia pelo request/event ID; leases stale são recuperáveis e side effects idempotentes.
- **Acceptance criteria negativos:** crash antes/depois de qualquer boundary não pode perder evento nem repetir efeito não autorizado; consumer duplicate não avança duas vezes.
- **Evidência necessária:** state/event schema, outbox/inbox design, fault-injection matrix, reconciliation records e SHA/tree/policy.
- **Condição de desbloqueio:** AB-012/AI-023 aceitos com crash matrix e política de unknown outcome; scheduler só abre após recovery.
- **Dependências:** DATA-001; SDD D-007 e §6; architecture-boundaries §§2.3, 8–9; AI-007/022–024; PR-023/176/187.
- **Status atual:** `PARTIAL/NO_PROOF` — durable workflow, event envelope, dedupe e recovery são normativos; atomicidade/outbox e evidence ainda não existem como prova.

### DATA-003

- **Seção/linhas:** `specification-review.md:181-191`; source: “Persistência e distribuição” e Milestone 16.
- **Evidência:** SQLx/migrations, backups, migration e rollback são requisitos a fechar, sem ordem, preconditions, lock, dry-run, checksum, downgrade ou partial recovery.
- **Risco:** update torna DB irrecuperável; binário anterior fica incompatível; instâncias aplicam migration em conflito.
- **Decisão normativa exigida:** versionamento monotônico, forward-only ou downgrade formal, backup obrigatório, preflight, profile lock, checksum, compatibility window e fail/rollback policy.
- **Artefato owner:** `ADR-AB-004` + `AB-016`/AI-027 + migration compatibility/restore matrix + torn-migration/power-loss fixture.
- **Owner de revisão:** Data/Migration reviewer independente; Release reviewer.
- **PRs/milestones afetados:** PR 025–026, 186–187; M16; releases v0.1–v1.0.
- **Acceptance criteria positivos:** cada migration tem manifest/checksum/preflight/backup/lock; upgrade suportado, restore e forward-fix são determinísticos.
- **Acceptance criteria negativos:** checksum drift, schema unsupported, lock concorrente, interrupção, downgrade sem migration validada ou binário incompatível deve resultar em `NO_GO` sem perder last-known-good.
- **Evidência necessária:** compatibility matrix por versão, dry-run/lock/checksum records, backup digest, interrupted/restore fixtures e artifact identity.
- **Condição de desbloqueio:** AB-016/AI-027 aceitos; updater e PRs de schema não podem avançar sem matrix e last-known-good.
- **Dependências:** DATA-001/002; SDD §6 e §8; architecture-boundaries §§8–9.3, 10; PR-025/026/254/255.
- **Status atual:** `PARTIAL/NO_PROOF` — regras de preflight, checksum, backup, transaction e downgrade estão descritas; matrix e execução não foram provadas.

### WF-001

- **Seção/linhas:** `specification-review.md:328-338`; source: “Workflow, scheduler e eventos”.
- **Evidência:** persistência, logs e crash recovery são requisitos, sem estado de node/run, checkpoint, lease, retry, compensation, idempotent side effect ou interrupted detection.
- **Risco:** workflow reporta sucesso incompleto, repete pagamento/commit ou permanece eternamente em `running`.
- **Decisão normativa exigida:** state machine de run/node, durable checkpoint, idempotency key por node, lease/heartbeat, recovery, compensation e `unknown/needs_review` para efeito ambíguo.
- **Artefato owner:** `ADR-AB-013` + contrato `AB-012` + workflow state/recovery matrix + crash/replay/unknown-outcome fixture.
- **Owner de revisão:** Workflow/Recovery reviewer independente; Security reviewer para side effects.
- **PRs/milestones afetados:** M10–M11, M16; PR 176–188, 195–200, hardening.
- **Acceptance criteria positivos:** DAG versionado retoma checkpoint seguro; leases/fencing e retries são bounded; efeito desconhecido vai para reconciliação.
- **Acceptance criteria negativos:** crash em cada transição não pode marcar sucesso falso, duplicar side effect, perder logs/checkpoint ou permanecer sem terminal state.
- **Evidência necessária:** state schema/transition table, checkpoint digest, lease/recovery records, fault matrix e exact SHA/tree/policy.
- **Condição de desbloqueio:** AB-012 aceito com crash/recovery contract; scheduler/automation dependente permanece bloqueado sem `PASS`.
- **Dependências:** DATA-001/002; SDD D-007; architecture-boundaries §§3.13, 9.2; AI-022/023/024; PR-176/187/195.
- **Status atual:** `PARTIAL/NO_PROOF` — DAG, checkpoint, recovery e no-replay estão normatizados; state machine e evidence executável não estão fechados.

### MA-001

- **Seção/linhas:** `specification-review.md:295-305`; source: “Domínio e políticas” e “Multi-Agent”.
- **Evidência:** `InvocationGraph` valida depth/cycles, mas não fan-out, total nodes/calls, concurrency, wall clock, memória, budget compartilhado, dedupe, lease ou identity/capability delegado.
- **Risco:** agent loop/storm/deadlock/confused deputy usa permissão do chamador fora do escopo.
- **Decisão normativa exigida:** execution envelope com depth/fan-out/total calls/wall clock/tokens/cost/concurrency; principal por node, capability narrowing, dedupe, lease e cancelamento da subtree.
- **Artefato owner:** `ADR-AB-012` + `AB-011`/AI-021 + invocation envelope/budget matrix + cycle/fanout/concurrency/cancel fixture.
- **Owner de revisão:** Multi-agent/Runtime reviewer independente; Security reviewer.
- **PRs/milestones afetados:** M9–M13; PR 160–169, 177–185, 218–231.
- **Acceptance criteria positivos:** graph validado antes de delegation/parallel; cada node tem principal/origin/budget; cancel root libera descendants, leases e budget.
- **Acceptance criteria negativos:** cycle, depth/fan-out/concurrency/budget overflow, cross-project target ou capability herdada sem narrowing deve terminar `blocked/rejected`.
- **Evidência necessária:** envelope schema, graph hash/provenance, budget reservations, cancellation/lease records e negative suite bound ao SHA.
- **Condição de desbloqueio:** AB-011 aceito e envelope aplicado a agent/workflow/tool delegation; nenhum multi-agent unattended antes do gate.
- **Dependências:** SEC-001/002; DATA-002; SDD §4; architecture-boundaries §3.12; AI-021/023/024; PR-160/162/177.
- **Status atual:** `PARTIAL/NO_PROOF` — depth/cycle/budget/fan-out aparecem na fronteira; envelope completo e provas de cancellation/dedup não existem.

### OBS-001

- **Seção/linhas:** `specification-review.md:475-485`; source: “Workflow, scheduler e eventos” e “Requisitos de qualidade”.
- **Evidência:** trace de execução inclui prompt assembly, request/response, tools, memória, skills, erros, usage, custo e duração sem classificação, redaction, retenção, acesso, encryption ou consent.
- **Risco:** trace exfiltra prompts, tokens, arquivos e credenciais ou impede uso em ambientes sensíveis.
- **Decisão normativa exigida:** separar audit mínimo de conteúdo detalhado; classificar sensibilidade, redigir antes de persistir, TTL/access/encryption/consent e export/delete seguros.
- **Artefato owner:** `ADR-AB-016` + contrato `AB-015` + trace/redaction/retention matrix + sensitive golden/sink-failure/delete fixtures.
- **Owner de revisão:** Observability/Privacy reviewer independente; Security reviewer.
- **PRs/milestones afetados:** M4, M5, M7–M15, M16; PR 095, 120, 188, 202, 215, 247, 259, hardening.
- **Acceptance criteria positivos:** spans e audit têm IDs, outcome, digest/policy/schema revision; conteúdo sensível é redigido e acesso exige capability.
- **Acceptance criteria negativos:** secret/PII/prompt integral/page content/raw headers em qualquer sink, retenção vencida ou sink failure tratado como sucesso deve falhar.
- **Evidência necessária:** schema, redaction golden corpus, retention/delete/access logs, encryption policy e sink-failure matrix, ligados ao SHA/tree/policy.
- **Condição de desbloqueio:** AB-015 aceito e traces/indices com redaction/retention comprováveis; atividade sem trace é `NO_PROOF`.
- **Dependências:** SEC-004; SDD §6–§7; architecture-boundaries §§2.3–2.4, 3.17, 10; AI-014/025/033; PR-095/188/259.
- **Status atual:** `PARTIAL/NO_PROOF` — TraceRecord, redaction, correlation e retention são descritos; classificação/consent e evidence por sink continuam abertas.

## W3 — Python, MCP, plugins, remoto e evolução

### EXT-001

- **Seção/linhas:** `specification-review.md:635-645`; source: “Segurança e execução” e Milestone 6 — Python Runtime.
- **Evidência:** Python sidecar JSON-RPC é opcional, mas framing, schema/version, handshake, timeout, cancel, restart, isolation, limits e dependency installation não estão definidos.
- **Risco:** worker travado, command injection, supply-chain de package, processo órfão ou bypass do Permission Engine.
- **Decisão normativa exigida:** protocolo versionado com framing/handshake/capability negotiation, IPC auth, bounded payloads, sandbox, quotas, venv/lockfile/allowlist, supervision e fail-closed.
- **Artefato owner:** `ADR-AB-008` + contrato `AB-007` + Python protocol/dependency matrix + malformed/restart/no-Python/permission fixtures.
- **Owner de revisão:** Python/Runtime reviewer independente; Security/Sandbox reviewer.
- **PRs/milestones afetados:** M5–M6, M10, M14–M15; PR 103, 112–121, 180, 233–243, 248.
- **Acceptance criteria positivos:** worker incompatível é rejeitado; core boota sem Python; crash/restart usa generation e não duplica RPC; package é pinned e mediado.
- **Acceptance criteria negativos:** JSON-RPC oversized/malformed, worker sem auth, dependency sem pin, stdout tratado como instrução ou RPC sem Permission path deve falhar.
- **Evidência necessária:** protocol schema/version, handshake trace, lifecycle/crash matrix, dependency lock/allowlist, no-Python fixture e SHA/tree/policy.
- **Condição de desbloqueio:** AB-007 aceito e core no-Python + isolation/permission fixtures com resultado verificável.
- **Dependências:** SEC-002/003; SDD D-006 e §5; architecture-boundaries §3.8, §5.2, §11; AI-016/017; PR-103/112/180.
- **Status atual:** `PARTIAL/NO_PROOF` — sidecar opcional, JSON-RPC, lifecycle e no-Python aparecem; framing/handshake/quota/dependency proof faltam.

### EXT-002

- **Seção/linhas:** `specification-review.md:646-656`; source: “Segurança e execução” e “MCP e Plugins”.
- **Evidência:** MCP stdio/HTTP e discovery são listados sem versão, handshake, origem, auth/TLS/pinning, schema validation, dynamic capability, prompt/resource access ou revoke.
- **Risco:** MCP malicioso registra tool, recebe dados de projeto ou muda schema durante run aprovado.
- **Decisão normativa exigida:** trust model, transport security, manifest/capabilities, approval por server/tool/version, payload validation, connection lifecycle, replay/revoke.
- **Artefato owner:** `ADR-AB-017` + contrato `AB-017` + MCP transport/auth/capability matrix + malicious-server/TLS/schema/disconnect fixtures.
- **Owner de revisão:** Extension/MCP Security reviewer independente.
- **PRs/milestones afetados:** M5, M14–M15; PR 096–111, 232–237, 244–251.
- **Acceptance criteria positivos:** server/tool/version são manifestados e aprovados; schema/capability drift exige reapproval; endpoint HTTP usa TLS/auth/pinning conforme policy.
- **Acceptance criteria negativos:** tool desconhecida, schema mudado, HTTP sem TLS quando requerido, replay, disconnect/retry permissivo ou origin não allowlisted deve ser negado/quarantined.
- **Evidência necessária:** manifest/schema/version records, handshake/auth traces, capability decision, revoke/quarantine and threat fixtures, artifact/SHA/policy.
- **Condição de desbloqueio:** AB-017 aceito, Permission Engine integrado e suite de threat MCP com fail-closed.
- **Dependências:** SEC-001/002/003; SDD §5; architecture-boundaries §3.18, §5.2, §11; AI-017/036; PR-232/236.
- **Status atual:** `PARTIAL/NO_PROOF` — opt-in, manifest, auth/transport, quarantine e revoke são normativos; detalhes de versão/approval e testes não têm prova.

### EXT-003

- **Seção/linhas:** `specification-review.md:657-667`; source: “Segurança e execução” e “MCP e Plugins”.
- **Evidência:** plugin pode registrar provider/tool/memory/workflow/connector/event handler sem decisão in/out-of-process, ABI/API, isolation, lifecycle, compatibility, signature ou permissions.
- **Risco:** plugin acessa processo/secrets do core; update corrompe DB/eventos ou quebra runtime.
- **Decisão normativa exigida:** tipos e isolation levels; protocolo versionado para código não confiável; manifest/signature/capabilities/resource limits/install-update-disable/migration hooks seguros.
- **Artefato owner:** `ADR-AB-017` + plugin manifest/lifecycle contract + plugin permission/isolation/compatibility matrix + malicious plugin/crash/revoke fixtures.
- **Owner de revisão:** Extension Architecture/Security reviewer independente.
- **PRs/milestones afetados:** M14–M16; PR 238–243, hardening.
- **Acceptance criteria positivos:** plugin autorizado carrega em profile declarado, com capability e versão verificadas; crash/timeout é isolado; revoke mata leases/handles.
- **Acceptance criteria negativos:** plugin sem signature/manifest, ABI incompatível, acesso direto ao core DB/secrets, update que muda estado ou capability transitiva deve ser rejeitado/quarantined.
- **Evidência necessária:** manifest/signature/provenance, process/ABI decision, lifecycle/revoke records, compatibility matrix and negative tests bound to artifact/SHA.
- **Condição de desbloqueio:** plugin boundary e quarantine/revoke contract aceitos; provider/tool plugins só após lifecycle/permission gates.
- **Dependências:** SEC-002/003/004; EXT-002; SDD §5; architecture-boundaries §§3.18, 5.2, 10; AI-017/036; PR-238/240/241.
- **Status atual:** `PARTIAL/NO_PROOF` — opt-in, manifest, allowlist, quarantine e rollback aparecem; decisão ABI/processo e evidence não estão fechadas.

### EXT-004

- **Seção/linhas:** `specification-review.md:668-678`; source: “Produto e objetivo” e “Remote Runtime”.
- **Evidência:** runtime transport, protocolo, daemon, WebSocket, remote tools/projects e credential isolation aparecem como backlog sem identity, trust boundary, enrollment, replay, reconnect/offline ou data residency.
- **Risco:** daemon remoto vira extensão privilegiada; disconnect duplica trabalho; credentials/projects cruzam nodes.
- **Decisão normativa exigida:** protocolo versionado, node identity/mTLS quando aplicável, enrollment/revoke, project/tool auth, nonces/replay, leases/reconnect e data/credential policy.
- **Artefato owner:** `ADR-AB-020` + contratos `AB-003`/`AB-017`/remote + node identity/credential isolation matrix + replay/disconnect/cross-project fixtures.
- **Owner de revisão:** Remote Security/Architecture reviewer independente.
- **PRs/milestones afetados:** M15–M16; PR 244–251 e hardening.
- **Acceptance criteria positivos:** enrolled node tem scope/lease; revoke remove acesso imediatamente; disconnect produz run recuperável; credentials são handles isolados.
- **Acceptance criteria negativos:** replay, node removido, project mismatch, credential crossing, reconnect duplicado ou offline fallback sem policy deve falhar fechado.
- **Evidência necessária:** protocol/identity schemas, enrollment/revoke logs, nonce/lease records, reconnect state matrix, data-residency policy and exact artifact/SHA.
- **Condição de desbloqueio:** AB-020 aceito com credential isolation e remote threat suite; PR-248/249/251 continuam após PR-250.
- **Dependências:** SEC-001/002/004; DATA-002; SDD §5; architecture-boundaries §§3.18, 5.2, 9.2–9.4, 11; PR-244–250.
- **Status atual:** `PARTIAL/NO_PROOF` — remote identity/protocol/credential isolation/revoke estão apontados; enrollment, replay, reconnect e residência não têm prova.

### EVOL-001

- **Seção/linhas:** `specification-review.md:703-713`; source: “Memória, skills e evolução”.
- **Evidência:** skill contém `SKILL.md`, scripts, templates, references e tests e pode ser criada/testada/ativada, sem classificação de arquivos, sandbox, dados/capabilities ou separação de conteúdo e instrução confiável.
- **Risco:** skill executa código, exfiltra secrets ou injeta instrução com poderes do agente.
- **Decisão normativa exigida:** classificar cada arquivo/capability; scripts somente via Tool Runtime + Sandbox; manifest/schema obrigatório; conteúdo é T0/T1 não confiável até promoção explícita.
- **Artefato owner:** `ADR-AB-010` + `AB-009` + skill manifest/file-classification matrix + script isolation/malicious-reference fixture.
- **Owner de revisão:** Skills/Security reviewer independente.
- **PRs/milestones afetados:** M5, M8, M13–M14; PR 136–154, 218–231, 238–243.
- **Acceptance criteria positivos:** skill version é imutável, digestada e pinada por run; script usa capability/profile explícitos; instalação/execução gera audit.
- **Acceptance criteria negativos:** skill sem manifest/capability, script fora do workspace, referência que rebaixa security ou import global implícito não ativa.
- **Evidência necessária:** manifest/digest, file-classification, sandbox/permission records, malicious prompt corpus, activation/rollback fixture and SHA/tree/policy.
- **Condição de desbloqueio:** AB-009/AB-006 aceitos; candidate só pode ativar após teste/evaluation/approval e last-known-good.
- **Dependências:** SEC-002/003; RUNTIME-001; SDD D-008; architecture-boundaries §3.10, 5.1, 8; AI-018/020; PR-136/142/149.
- **Status atual:** `PARTIAL/NO_PROOF` — skill lifecycle, immutable version, sandbox e rollback são normativos; classificação completa e prova de execução mediada faltam.

### EVOL-002

- **Seção/linhas:** `specification-review.md:714-724`; source: “Memória, skills e evolução”.
- **Evidência:** L0–L4 usam frases como “sugere”, “cria/testa”, “ativa após testes” e “altera dentro dos limites”, sem limits, approver, branch policy, evidence ou forbidden action.
- **Risco:** L3/L4 ativa mudança de security/workflow/config sem review humano ou com interpretação divergente.
- **Decisão normativa exigida:** capability matrix e gate por nível: propor, modificar, testar, publicar, promover, reverter; security/permission/provider/release exigem aprovação independente.
- **Artefato owner:** `ADR-AB-010` + capability/autonomy matrix + proposal/evaluation/rollback contract + L0–L4 negative fixture.
- **Owner de revisão:** Governance/Autonomy reviewer independente; Security reviewer para high-risk transitions.
- **PRs/milestones afetados:** M8, M12–M13, M16; PR 149–154, 204–231.
- **Acceptance criteria positivos:** cada transição tem preconditions, approver, scope, branch, tests e rollback; L3/L4 são limitados a capability explicitamente concedida.
- **Acceptance criteria negativos:** L0–L2 não ativam, L3/L4 não publicam security/permission/release sem approval, e proposta fora do scope não cria PR/efeito.
- **Evidência necessária:** matrix versionada, proposal/evaluation records, distinct approver identity, regression/rollback fixtures and policy revision.
- **Condição de desbloqueio:** autonomy gate aceito e transições negativas reproduzíveis antes de qualquer rollout automático.
- **Dependências:** GOV-003; SEC-002; RUNTIME-001; SDD D-008 e releases v0.8; architecture-boundaries §§3.10–3.12, 5.1; PR-149/227/228.
- **Status atual:** `PARTIAL/NO_PROOF` — níveis e proposta→testes→approval→rollback são mencionados; matrix e evidência por transição permanecem abertas.

## W4 — Testes adversariais, distribuição e release

### TEST-002

- **Seção/linhas:** `specification-review.md:532-542`; source: “Segurança e execução” e “Requisitos de qualidade”.
- **Evidência:** fuzz/security/load são listados sem threat cases, propriedades, boundaries ou ambiente; faltam cross-project, escape, confused deputy, secret leakage e fail-closed.
- **Risco:** security gate nominal não detecta os riscos centrais.
- **Decisão normativa exigida:** threat-model test plan com casos positivos/negativos para IPC, tools, filesystem, shell, Python, MCP, plugin, remote, secrets, injection e migrations.
- **Artefato owner:** `AB-019`/AI-034/AI-037 + threat regression manifest + security test matrix + adversarial/negative fixture corpus.
- **Owner de revisão:** Security/QA reviewer independente; Architecture reviewer para boundaries.
- **PRs/milestones afetados:** M2, M5–M6, M8–M9, M12–M15, M16; PR 044, 096–121, 136–154, 232–251.
- **Acceptance criteria positivos:** cada threat alta tem teste automatizado, artifact reproduzível e owner; gate mapeia ameaça→controle→caso→release.
- **Acceptance criteria negativos:** authorization/sandbox/secret/replay/cross-project failure, skipped/disabled case, stale evidence ou security CI failure bloqueia release.
- **Evidência necessária:** machine-readable threat manifest, fixtures, tool/OS versions, logs redigidos, result status, SHA/tree/policy e exception formal quando aplicável.
- **Condição de desbloqueio:** threat matrix completa e ligada aos cards/AI invariants; só `PASS` atual e independente libera capability/release.
- **Dependências:** SEC-001..004, DATA-002/003, EXT-001..004, GOV-003; SDD §7; architecture-boundaries §5.2/§11–§13; PR-260–265.
- **Status atual:** `PARTIAL/NO_PROOF` — SDD, AI e threat-boundaries enumeram controles/casos; não há manifest executado ou prova por ambiente.

### DIST-001

- **Seção/linhas:** `specification-review.md:589-599`; source: “Persistência e distribuição”.
- **Evidência:** packaging/signing/installer são requisitos a fechar sem publisher identity, trust chain, timestamp, certificates, rotation, channels ou artifacts por OS/arch.
- **Risco:** installer não confiável, chave não revogável e build diferente do testado.
- **Decisão normativa exigida:** build supply chain, signing/notarization por plataforma, key custody/rotation/revoke, provenance/SBOM, channels e install/update verification.
- **Artefato owner:** `ADR-AB-019` + contrato `AB-018` + release artifact/provenance/SBOM matrix + wrong-signer/digest/OS fixture.
- **Owner de revisão:** Release/Distribution reviewer independente; Security reviewer de supply chain.
- **PRs/milestones afetados:** M0, M16; PR 002, 004, 016 e hardening de release.
- **Acceptance criteria positivos:** artifact por OS/arch é identificável, digestado, SBOM/provenance-bound e verificado em clean install; keys ficam fora de PR/fork/agent.
- **Acceptance criteria negativos:** signer/channel/OS/arch/SHA/tree/policy mismatch, unsigned metadata, expired/revoked key ou artifact substituído impede instalação.
- **Evidência necessária:** protected workflow manifest, key custody/rotation policy, signed artifact metadata, SBOM, provenance attestation e clean-install verifier results.
- **Condição de desbloqueio:** AB-018/019 aceitos e verifier negativo reproduzível; nenhum claim de release assinado sem tuple exata.
- **Dependências:** ARCH-001/002, SEC-004, GOV-003, TEST-002; SDD D-009 e §6–§7; architecture-boundaries §10–§12; PR-016/266/267.
- **Status atual:** `PARTIAL/NO_PROOF` — checksums, SBOM, provenance, signature e install smoke estão normatizados; publisher/key/artifact evidence não foi produzido.

### DIST-002

- **Seção/linhas:** `specification-review.md:600-610`; source: “Persistência e distribuição” e “Releases alvo”.
- **Evidência:** update/rollback são citados sem atomicidade, channel/pin, staged rollout, signed manifest, schema compatibility, interrupted download ou fallback.
- **Risco:** update parcial, downgrade inseguro, incompatibilidade app↔DB/skills/workflows ou rollout comprometido.
- **Decisão normativa exigida:** signed manifest, verify-before-apply, staging/A-B atômico, health check, last-known-good, backup e compatibility matrix.
- **Artefato owner:** `ADR-AB-019` + update/rollback contract `AB-018` + app↔schema↔sidecar compatibility matrix + power-loss/network/signature/health fixtures.
- **Owner de revisão:** Updater/Release reviewer independente; Data/Migration and Security reviewers.
- **PRs/milestones afetados:** M1, M6, M10–M11, M14–M16; PR 016, 025–026, 119–121, 186–203, 232–251.
- **Acceptance criteria positivos:** valid manifest é staged e verificado antes de substituir; interruption preserva versão atual/profile; health failure reverte para last-known-good.
- **Acceptance criteria negativos:** invalid signature/hash/channel/expiry/min-version, power loss, disk full, interrupted download, schema incompatível ou revoked version deve manter versão anterior intacta.
- **Evidência necessária:** signed metadata, staging state, compatibility manifest, backup digest, health/rollback records e drill matrix no mesmo artifact/SHA/policy.
- **Condição de desbloqueio:** updater e rollback contracts aceitos; PR-268/269 só liberam distribuição após drill independente.
- **Dependências:** DATA-003, SEC-004, DIST-001, TEST-002; SDD D-009 e §6; architecture-boundaries §§8–10; AI-027–029; PR-255/266–269.
- **Status atual:** `PARTIAL/NO_PROOF` — verification, last-known-good e fail-closed estão descritos; política completa de staging/compatibilidade e drill não têm prova.

### ROAD-001

- **Seção/linhas:** `specification-review.md:850-860`; source: “Releases alvo” e entregável esperado.
- **Evidência:** releases são apenas associações de milestones, sem demo, acceptance, non-goals, security/data/compatibility criteria ou ship/no-ship.
- **Risco:** milestone “concluída” sem produto demonstrável ou v1.0 anunciada sem update/recovery/signing provados.
- **Decisão normativa exigida:** release contract por versão, com demo reproduzível, required gates, support/compatibility matrix, non-goals, accepted risks, rollback e evidência mínima; decisão final só pode ser humana/protegida.
- **Artefato owner:** `releases.md`/release contract + `AB-018`/`AB-019` + release checklist/support matrix + `NO_GO`/eligible-for-human-decision fixture.
- **Owner de revisão:** Release/Distribution reviewer independente, com Security, Architecture e QA sign-off distintos.
- **PRs/milestones afetados:** releases v0.1–v1.0; PR 001–251 e M16.
- **Acceptance criteria positivos:** cada release tem demo com artifact instalado e dados reproduzíveis, gates required, support matrix, residual-risk register e checklist assinado por reviewers distintos.
- **Acceptance criteria negativos:** missing evidence, failed/skipped/timeout/stale check, artifact/SHA/policy mismatch, unsupported OS/data, AI comment ou approval ausente resulta `NO_GO`.
- **Evidência necessária:** release manifest, demo artifact/digest, test/security/architecture/distribution reports, compatibility/rollback evidence, SHA/tree/policy e decisão humana/protegida.
- **Condição de desbloqueio:** contrato de release aceito e evaluator produzir somente `NO_GO` ou `eligible-for-human-decision`; não autoriza publicação automática.
- **Dependências:** todos os blockers cross-cutting; SDD §§1, 6–7 e releases v0.1–v1.0; architecture-boundaries §§10–13; AI-028/029/033/037; PR-270.
- **Status atual:** `PARTIAL/NO_PROOF` — SDD define compromissos observáveis por versão e gates de artifact; contract/checklist/demo e decisão ship/no-ship completos ainda não têm evidência.

## Limitações e regra final

O plano é uma consolidação normativa baseada exclusivamente nos arquivos-fonte indicados. A existência de um ADR, contrato, matriz ou fixture acima é uma condição futura de fechamento, não uma afirmação de que o artefato já existe ou foi executado. Qualquer resultado assíncrono, rebase ou mudança de fila deve revalidar SHA/tree/policy e reabrir o blocker afetado quando a identidade da evidência mudar.
