# Specification Review — SDD Input Pack da Plataforma Desktop Multiagente

## 1. Escopo e base da revisão

- **Fonte revisada:** `C:/Users/Gabriel/Desktop/Hank/.planning/source/sdd-input.md`.
- **Versão declarada da fonte:** Draft v0.1, fornecido em 2026-08-17.
- **Natureza desta entrega:** revisão de especificação e planejamento. Não há implementação, execução de produto, prova de comportamento ou aprovação de release nesta revisão.
- **Método:** inspeção de requisitos explícitos, fronteiras arquiteturais, contratos, invariantes, riscos de segurança, persistência, distribuição, observabilidade, testes, extensões e execução por agentes.
- **Regra de classificação:** cada achado possui uma natureza primária — **contradição explícita**, **ambiguidade** ou **requisito ausente** — e exatamente uma severidade: `BLOCKER`, `MAJOR`, `MINOR` ou `SUGGESTION`.

## 2. Veredito executivo

**Veredito: Not ready.**

O material é um bom inventário de intenção e de possíveis componentes, mas ainda não é um SDD executável. Há uma contradição de fronteira no enunciado do núcleo (Tauri é descrito como núcleo e depois explicitamente excluído do Agent Core), não existem contratos normativos suficientes para API, eventos, providers, tools, workflows, extensões ou remoto, e os controles de identidade, autorização, sandbox, segredos, migração, recuperação e atualização são apenas direções. O inventário de PRs não contém dependências verificáveis, critérios de aceite, artefatos, ondas, caminho crítico ou a fila detalhada de hardening.

Os `BLOCKER`s precisam ser decididos antes da seleção de PRs de implementação. `MAJOR`s podem ser fechados por ADR/especificação complementar antes da respectiva milestone; `MINOR`s e `SUGGESTION`s não devem ser usados para mascarar decisões de segurança, dados ou release.

## 3. Achados

### 3.1 Arquitetura, camadas e escopo

#### ARCH-001

- **Severidade:** `BLOCKER`
- **Natureza:** contradição explícita
- **Seção do source:** “Produto e objetivo”; “Arquitetura normativa proposta”; “Regras imutáveis da fonte”, item 1.
- **Evidência/omissão:** o produto diz “Rust + Tauri 2 como núcleo” e descreve “Core Rust modular + Tauri”, mas a regra imutável diz “Tauri não é Agent Core”. Não está definido se Tauri é shell/adaptador, processo de composição, API host ou parte do núcleo.
- **Risco:** uma implementação pode colocar estado, regras de segurança ou runtime no processo/UI Tauri, quebrando reutilização por CLI, TUI, API e workers e criando uma fronteira de confiança diferente por superfície.
- **Correção normativa:** declarar formalmente `agent-core`/runtime como camada independente de Tauri; Tauri deve ser apenas shell/adaptador de transporte, eventos e janela. Definir direção permitida de dependências e proibir qualquer acesso de domínio ao Tauri.
- **Critério de aceite sugerido:** ADR e diagrama nomeiam os processos/camadas, seus contratos e dependências permitidas; um teste de arquitetura falha se um crate core importar Tauri/UI; uma operação equivalente pode ser exercida por um adaptador não-Tauri.
- **Dependências afetadas:** M0; PR 001–003, 018–019; toda a cadeia M1–M15.

#### ARCH-002

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Arquitetura normativa proposta”; inventário `apps/desktop`, `apps/cli` e crates.
- **Evidência/omissão:** os nomes de crates e adapters são listados, mas não há matriz de ownership, dependências permitidas, ciclo de vida, fronteira de processo ou contrato entre Application API, Agent Runtime, storage, event bus e adapters.
- **Risco:** ciclos entre crates, vazamento de provider/storage para o core e múltiplas regras de autorização para a mesma operação; agentes diferentes podem implementar contratos incompatíveis.
- **Correção normativa:** acrescentar matriz “camada → responsabilidade → dependências permitidas → dados que pode conhecer → erros que pode emitir”, incluindo processo, threading/async, lifecycle e política de compatibilidade.
- **Critério de aceite sugerido:** a matriz cobre todos os crates propostos; `cargo`/lint de arquitetura verifica pelo menos as fronteiras core↔provider, core↔Tauri, core↔browser-core e UI↔SQLite; cada comando da API tem owner único.
- **Dependências afetadas:** PR 001, 018–019, 021–024, 056–060, 096–099 e todas as milestones.

#### ARCH-003

- **Severidade:** `MAJOR`
- **Natureza:** ambiguidade
- **Seção do source:** “Produto e objetivo”; “Arquitetura normativa proposta”.
- **Evidência/omissão:** o core deve ser reutilizável por Desktop, CLI, TUI, Web, Mobile, API e workers remotos, porém apenas Desktop e `apps/cli` aparecem no workspace, e o CLI é qualificado como “futuro”. Não há definição do subconjunto suportado em v0.1–v1.0.
- **Risco:** a equipe pode criar abstrações prematuras ou prometer paridade inexistente; mudanças no protocolo local podem inviabilizar web/mobile/remoto.
- **Correção normativa:** fixar superfícies de v1, superfícies experimentais e não-goals; para cada uma, indicar transporte, autenticação, streaming, disponibilidade offline e compatibilidade.
- **Critério de aceite sugerido:** a especificação possui uma tabela de superfícies com escopo de release e um teste de contrato para as superfícies declaradas como suportadas.
- **Dependências afetadas:** M0, M4, M9–M15; PR 001–003, 089–091, 244–251.

#### ARCH-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Arquitetura normativa proposta”; “Runtime, providers e contexto”.
- **Evidência/omissão:** “microsserviços locais são evolução posterior” e “core modular” são decisões, mas não há critério para o que é in-process, sidecar, daemon ou remoto, nem quais contratos devem permanecer estáveis durante a evolução.
- **Risco:** um sidecar ou worker pode adquirir privilégios diferentes sem revisão de threat boundary; a migração para remoto poderá exigir reescrita do runtime.
- **Correção normativa:** documentar um modelo de processos e trust boundaries para v1, incluindo IPC, transportes, ownership de dados, timeouts, backpressure, shutdown e compatibilidade futura.
- **Critério de aceite sugerido:** threat model e arquitetura mostram UI, core, sidecar, provider, sandbox e remoto como nós distintos, com fluxos e controles explícitos.
- **Dependências afetadas:** M0, M3, M5–M6, M14–M15; PR 002, 112–121, 232–251.

#### ARCH-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Produto e objetivo”; “Entregável esperado do planejamento”.
- **Evidência/omissão:** o domínio inclui projetos, agentes, grupos, sessões, memória, skills, workflows, automações, tarefas, arquivos, repositórios, permissões, orçamento, contexto, providers, Python, MCP, plugins e remoto, mas não há escopo normativo mínimo nem não-goals por release.
- **Risco:** scope creep, contratos provisórios tratados como definitivos e impossibilidade de demonstrar uma release verificável.
- **Correção normativa:** transformar cada release em capacidades observáveis, não-goals, critérios de entrada/saída e restrições de segurança; mover ideias sem contrato para backlog explicitamente não comprometido.
- **Critério de aceite sugerido:** cada release possui demo verificável, cenários de sucesso/falha, limites e lista de itens excluídos; nenhum PR pode depender de item sem contrato.
- **Dependências afetadas:** ROADMAP inteiro; PR 001–251 e hardening M16.

### 3.2 Identidade, autorização e segurança

#### SEC-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Domínio e políticas”; “Segurança e execução”.
- **Evidência/omissão:** o texto exige isolamento por projeto e acessibilidade/permissão em delegações, mas não define principal, identidade de usuário/agente/worker/plugin, escopo de recurso, tenancy, sessão de autorização ou propagação de contexto.
- **Risco:** acesso cruzado entre projetos, confused deputy em delegações e comandos que funcionam por um caminho (UI) mas bypassam a regra por outro (CLI, workflow, MCP ou remoto).
- **Correção normativa:** definir principal, subject, resource, action, project binding, capability token, origem, delegação e revogação; toda operação deve receber contexto de autorização verificável e fail-closed.
- **Critério de aceite sugerido:** matriz de autorização cobre cada entidade e cada superfície; testes negativos tentam acessar projeto, arquivo, memória, segredo e ferramenta de outro principal.
- **Dependências afetadas:** M1–M15; PR 027–035, 044, 068–071, 096–111, 155–172, 232–251.

#### SEC-002

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Regras imutáveis”, itens 5 e 6.
- **Evidência/omissão:** há nomes de políticas `always_allow/ask_once/ask_every_time/deny` e níveis `trusted/restricted/isolated`, mas não existe schema de capability, escopo, precedência, expiração, decisão para chamadas compostas, modo não-interativo ou comportamento quando o engine falha.
- **Risco:** uma ferramenta, workflow, plugin ou agente delegado pode herdar mais poder do que o usuário autorizou; falhas podem abrir acesso em vez de negar.
- **Correção normativa:** especificar capability por ação/recurso/projeto/principal, default deny, fail-closed, TTL, revogação, aprovação vinculada ao hash da operação, não-interatividade e auditoria.
- **Critério de aceite sugerido:** cada tool call produz decisão explicável; testes cobrem deny, timeout do aprovador, revogação, delegação, retry, execução paralela e ausência de UI; nenhum erro de policy executa a ação.
- **Dependências afetadas:** M2, M5–M6, M9–M15; PR 044, 096–111, 121, 161, 236, 241, 248.

#### SEC-003

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”.
- **Evidência/omissão:** `trusted/restricted/isolated` e futuras opções Docker/Podman/SSH/remote/WASM são citadas sem definir filesystem roots, symlinks, rede, processos filhos, ambiente, limites de CPU/memória/disco, usuário, seccomp/AppContainer/sandbox equivalente ou fallback quando a plataforma não oferece isolamento.
- **Risco:** “restricted” pode ser apenas uma promessa; terminal, Python, MCP e plugins podem escapar do projeto ou do host.
- **Correção normativa:** definir perfis de sandbox por OS e por ferramenta, capabilities concedidas, limites, telemetria, falhas e política de indisponibilidade. Nenhum perfil deve degradar silenciosamente para shell irrestrito.
- **Critério de aceite sugerido:** matriz de sandbox para Windows/Linux/macOS com testes de escape, path traversal, symlink, rede, processo e consumo; instalação falha ou pede decisão explícita se o isolamento obrigatório não existir.
- **Dependências afetadas:** M5–M6, M14–M15, M16; PR 103–105, 118–121, 233–243, 247–250.

#### SEC-004

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Persistência e distribuição”.
- **Evidência/omissão:** OS keychain/Tauri Stronghold é indicado, porém não há ciclo de vida de credenciais, rotação/revogação, escopo por provider/projeto, exportação, backup, restauração, crash dump, clipboard, memória, cache, artifacts ou comportamento ao perder o keychain.
- **Risco:** vazamento de tokens em logs, traces, backups ou artifacts; impossibilidade de recuperar ou invalidar credenciais; migração que reintroduz plaintext.
- **Correção normativa:** definir secret envelope, armazenamento por plataforma, rotação, redaction central, export/import, backup criptografado, revogação e fail-closed; Stronghold/keychain não pode ser confundido com política completa.
- **Critério de aceite sugerido:** varredura automatizada e testes de logs/traces/backups/artifacts não encontram valores secretos; rotação e restauração preservam apenas referências criptografadas; perda de keychain pede reautenticação.
- **Dependências afetadas:** M3, M4, M6, M12, M14–M16; PR 068–071, 095, 120, 215–217, 250.

#### SEC-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”, OAuth.
- **Evidência/omissão:** browser/deep link/callback/token exchange são mencionados sem state/nonce, PKCE, redirect allowlist, binding ao usuário/projeto, proteção contra replay, expiração e tratamento de callbacks concorrentes.
- **Risco:** account-linking indevido, interceptação de callback ou associação do token ao projeto/principal errado.
- **Correção normativa:** especificar fluxos OAuth por provider com state/PKCE/nonce, allowlist de redirect, expiração, troca no core e armazenamento somente no serviço de segredos.
- **Critério de aceite sugerido:** testes rejeitam callback sem/duplicado/expirado, state divergente, provider errado e redirect não autorizado; nenhum token passa pela UI ou log.
- **Dependências afetadas:** M3; PR 070–073, 237, 250.

#### SEC-006

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Memória, skills e evolução”; “MCP e Plugins”.
- **Evidência/omissão:** skills têm scripts, plugins podem registrar componentes e MCP/Python podem executar código, mas não há modelo de proveniência, assinatura, revisão, allowlist de origem, pin de versão ou quarentena.
- **Risco:** supply-chain attack, skill maliciosa, servidor MCP comprometido ou dependência Python adulterada obtém credenciais e acesso a projetos.
- **Correção normativa:** definir trust tier e ciclo de instalação para cada extensão, manifest assinado/verificado, origem permitida, hash/pin, revisão e capabilities explícitas; instalação deve ser revogável e auditável.
- **Critério de aceite sugerido:** artefato sem origem/hashes/assinatura conforme política não ativa; update incompatível fica em quarentena; testes validam revogação e downgrade seguro.
- **Dependências afetadas:** M6, M8, M14–M15; PR 119, 136–154, 232–243, 246–251.

#### SEC-007

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Requisitos de qualidade”.
- **Evidência/omissão:** ações destrutivas, pagamentos, credentials e instalação podem exigir aprovação, porém não há catálogo de ações destrutivas, confirmação contextual, double-submit protection, identidade do aprovador, expiração ou auditoria.
- **Risco:** aprovação de uma operação é reutilizada em outra, a UI exibe contexto truncado ou um agente aprova sua própria ação.
- **Correção normativa:** definir request de approval imutável com principal, recurso, parâmetros normalizados, hash, risco, TTL, aprovador autorizado e resultado; separar sugestão, aprovação e execução.
- **Critério de aceite sugerido:** aprovação vinculada ao hash não pode ser reutilizada após alteração; timeout/fechamento nega; agente não pode aprovar sua própria ação sem política explícita.
- **Dependências afetadas:** M2, M5, M9–M13; PR 055, 111, 167–169, 184, 221–231.

### 3.3 Persistência, consistência e migrações

#### DATA-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”.
- **Evidência/omissão:** entidades e tabelas são enumeradas, mas não há schema normativo: IDs, chaves, constraints, versionamento otimista, timestamps, soft delete, ordenação, unicidade, estados válidos, tenancy ou retenção.
- **Risco:** implementações incompatíveis e perda silenciosa de estado, especialmente para sessões, runs, uso, tool calls e skills versionadas.
- **Correção normativa:** definir modelo lógico por entidade, invariantes, lifecycle, cardinalidade, índices, UTC, IDs globais, concorrência e regras de exclusão/arquivamento.
- **Critério de aceite sugerido:** migrations reproduzem o schema a partir do zero; testes de constraints rejeitam estados inválidos; repositories expõem somente transições permitidas e são cobertos por contract tests.
- **Dependências afetadas:** M1, M4, M7–M11, M13; PR 021–035, 078–081, 122–135, 173–203, 218–231.

#### DATA-002

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Workflow, scheduler e eventos”; “Persistência e distribuição”.
- **Evidência/omissão:** workflows devem sobreviver a restart e recovery após crash, e existe Event Bus, mas não há semântica de transação, outbox/inbox, ordering, entrega, deduplicação, idempotência, lease ou reconciliação.
- **Risco:** tool/workflow executado duas vezes, evento perdido após commit, run preso em estado intermediário ou custo duplicado após restart.
- **Correção normativa:** definir máquina de estados durável, atomicidade entre mudança e evento, IDs de execução, idempotency keys, leases, retry, compensação e recuperação de cada node/tool.
- **Critério de aceite sugerido:** testes injetam crash antes/depois de cada boundary; após restart não há evento perdido nem execução duplicada não autorizada; runs presos são detectados e recuperáveis.
- **Dependências afetadas:** M1, M4, M9–M11; PR 023–026, 084–090, 176–188, 195–200.

#### DATA-003

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”; “Milestone 16”.
- **Evidência/omissão:** SQLx/migrations, backups, migração de dados e rollback aparecem como requisitos a fechar, mas não há política de compatibilidade, ordem, pré-condições, lock, dry-run, checksum, downgrade ou recuperação de migration parcial.
- **Risco:** update torna banco irrecuperável, rollback de binário incompatível com schema ou duas instâncias aplicam migrations em conflito.
- **Correção normativa:** definir versionamento monotônico, migrations forward-only ou downgrade formal, backup obrigatório, preflight, lock, checksum, janela de compatibilidade e política de falha/rollback.
- **Critério de aceite sugerido:** matriz testa upgrade de toda versão suportada, falha no meio, restore e binário anterior; o updater não inicia código incompatível sem migration concluída.
- **Dependências afetadas:** PR 025–026, 186–187; M16; releases v0.1–v1.0.

#### DATA-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”.
- **Evidência/omissão:** blobs são distribuídos em diretórios por categoria, mas não há formato, checksum, atomicidade, quota, limpeza, orphan recovery, criptografia ou relacionamento entre blob e metadata.
- **Risco:** artifacts corrompidos, crescimento ilimitado, referências quebradas e cópia parcial durante crash/backup.
- **Correção normativa:** definir blob store com content hash ou ID, escrita temporária + rename atômico, checksum, quota/retention, GC referencial, recuperação e política de criptografia.
- **Critério de aceite sugerido:** teste de interrupção durante escrita/cópia não deixa referência válida para conteúdo parcial; GC não remove blob referenciado; quota e erro de disco são observáveis.
- **Dependências afetadas:** M4, M7–M8, M10–M13; PR 080–081, 132–134, 143–153, 188–190, 218–231.

#### DATA-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Memória, skills e evolução”; “Persistência e distribuição”.
- **Evidência/omissão:** mensagens brutas, memórias, embeddings, traces, logs, custos e artifacts podem conter dados pessoais e segredos, mas não há retenção, exportação, eliminação, anonimização, residency ou TTL.
- **Risco:** retenção indefinida, exposição em index/vector store e impossibilidade de cumprir uma solicitação de apagamento.
- **Correção normativa:** definir classificação de dados, owner, retenção por tipo, delete cascade, reindexação, export/import e limites de armazenamento; separar telemetria operacional de conteúdo do usuário.
- **Critério de aceite sugerido:** apagar/arquivar projeto remove ou torna irrecuperáveis dados definidos na política; buscas e embeddings não retornam dados removidos; exportação é completa e auditada.
- **Dependências afetadas:** M4, M7, M10, M16; PR 079–081, 128–135, 188, 202, hardening.

#### DATA-006

- **Severidade:** `MAJOR`
- **Natureza:** ambiguidade
- **Seção do source:** “Persistência e distribuição”; “Produto e objetivo”.
- **Evidência/omissão:** SQLite é inicial e workers/remoto são previstos, mas não está dito se há uma única instância, múltiplos processos locais, acesso concorrente a arquivos ou sincronização entre nós.
- **Risco:** locks inadequados, corrupção, duas schedulers executando o mesmo job e divergência entre estado local e remoto.
- **Correção normativa:** declarar modelo de concorrência suportado em v1 e separar storage local de storage compartilhado; definir leader election/lease se houver múltiplos workers.
- **Critério de aceite sugerido:** cenários suportados têm testes de concorrência e restart; cenários não suportados são bloqueados pelo produto com mensagem explícita.
- **Dependências afetadas:** M1, M10–M11, M15; PR 025–026, 195–200, 244–251.

### 3.4 Runtime, contexto, providers e contratos internos

#### RUNTIME-001

- **Severidade:** `BLOCKER`
- **Natureza:** ambiguidade
- **Seção do source:** “Domínio e políticas”, hierarquia de instruções.
- **Evidência/omissão:** a ordem `system -> security -> project -> agent -> workflow -> skill -> conversation -> user` é listada, mas não se define se a ordem é precedência de override, ordem de montagem ou prioridade de conflito. Também não se diz que security não pode ser rebaixada por conteúdo posterior.
- **Risco:** prompt injection via user, conversation, skill, arquivo ou memória substitui restrições; dois agentes terão comportamentos divergentes.
- **Correção normativa:** definir merge/override por campo, camadas não sobrescrevíveis, origem confiável, delimitação de dados não confiáveis e validação antes do prompt.
- **Critério de aceite sugerido:** uma tabela normativa define precedência por campo; testes de injection não conseguem remover uma policy de segurança, capability ou isolamento.
- **Dependências afetadas:** M2, M4, M7–M9, M13; PR 043, 082–084, 131, 142, 159–169, 220–228.

#### RUNTIME-002

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Runtime, providers e contexto”.
- **Evidência/omissão:** `ModelProvider` deve normalizar stream/complete, capabilities, custo e erros, mas não há schemas versionados para request, response, chunks, finish reasons, usage, tool calls, multimodalidade, cancellation ou erro recuperável.
- **Risco:** cada adapter inventa semântica de streaming, custos ou tool calls; fallback e UI não conseguem distinguir saída parcial, final, cancelada e falha.
- **Correção normativa:** definir protocolo tipado e versionado, invariantes de stream, state machine de request, error taxonomy, cancellation token e normalização de usage/custo.
- **Critério de aceite sugerido:** adapters passam ao mesmo contract suite; uma stream pode ser reconstruída de eventos; cancellation e provider error têm resultado determinístico.
- **Dependências afetadas:** M3–M4; PR 056–060, 061–076, 085–090.

#### RUNTIME-003

- **Severidade:** `MAJOR`
- **Natureza:** ambiguidade
- **Seção do source:** “Runtime, providers e contexto”.
- **Evidência/omissão:** fallback para 429, timeout, outage e quota é requerido, mas não há ordem de fallback, limites, backoff, jitter, critério de retry, tratamento de stream parcial, custo de tentativas ou proibição de repetir tool call.
- **Risco:** tempestade de requests, custo inesperado, resposta duplicada ou side effect repetido ao trocar de provider.
- **Correção normativa:** definir retry budget, backoff, circuit breaker, idempotency, fallback por capability e política para requests com side effects/stream parcial.
- **Critério de aceite sugerido:** testes determinísticos verificam que um erro recuperável usa no máximo N tentativas, respeita budget e nunca repete uma ação não idempotente sem nova aprovação.
- **Dependências afetadas:** M3–M5, M9–M11; PR 075–076, 087, 099, 110, 166, 177–188.

#### RUNTIME-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Runtime, providers e contexto”; “Memória, skills e evolução”.
- **Evidência/omissão:** context builder escolhe memórias, skills, arquivos, tarefas e group context, mas não define classificação de confiabilidade, proteção contra prompt injection, redaction, truncamento, ordem, custo ou explicação do que foi enviado.
- **Risco:** conteúdo não confiável instrui o agente, dados de outro projeto entram no prompt, tokens/custos explodem e o usuário não consegue auditar uma decisão.
- **Correção normativa:** definir pipeline de contexto com provenance, trust labels, filtros de projeto, redaction, orçamento por fonte, truncamento determinístico e modo de inspeção seguro.
- **Critério de aceite sugerido:** cada request registra fontes e contagens sem revelar segredos; testes de cross-project/injection e overflow de contexto são rejeitados ou reduzidos conforme a policy.
- **Dependências afetadas:** M2, M4, M7–M9; PR 082–083, 125–131, 142, 159, 168–169.

#### RUNTIME-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Runtime, providers e contexto”.
- **Evidência/omissão:** há budget por projeto/agente/workflow/task com tokens e custo, mas não há fonte de verdade, moeda/unidade, reserving, concorrência, comportamento no limite, reconciliação com provider ou budget para retries/delegações.
- **Risco:** overspend por execuções paralelas, cobrança incorreta, bypass em fallback e workflows que continuam depois do limite.
- **Correção normativa:** definir ledger de usage, reserva/commit/release, limites hard/soft, custo desconhecido, hierarquia de budgets e atomicidade entre autorização e consumo.
- **Critério de aceite sugerido:** testes paralelos não ultrapassam hard limit; um run no limite é encerrado de forma segura; usage reconciliado é reproduzível a partir de eventos duráveis.
- **Dependências afetadas:** M3–M4, M9–M11; PR 047, 073, 095, 166, 190, 202.

#### RUNTIME-006

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Runtime, providers e contexto”.
- **Evidência/omissão:** routing por complexidade e modalidade e `capabilities` são citados, sem taxonomia de capability, preferência, incompatibilidade, disponibilidade, custo, latência ou decisão auditável.
- **Risco:** o router seleciona modelo sem suporte a tool/multimodalidade/context window, ou alterna de modelo de forma não reprodutível.
- **Correção normativa:** definir capability schema, modelo de seleção, fallback e motivo de decisão; registrar versão do catálogo e policy usada.
- **Critério de aceite sugerido:** MockProvider testa todas as combinações; uma requisição incompatível é rejeitada antes da chamada; trace registra seleção e motivo.
- **Dependências afetadas:** M3–M4; PR 057, 067, 073–076, 094–095.

### 3.5 Multiagente, loops e workflows

#### MA-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Domínio e políticas”; “Multi-Agent”.
- **Evidência/omissão:** `InvocationGraph` deve validar profundidade e ciclos, mas não há limite de fan-out, quantidade total de nodes, concorrência, tempo, memória, budget compartilhado, deduplicação, leases ou identidade/capability do agente delegado.
- **Risco:** loop de agentes, storm de delegações, deadlock ou confused deputy que usa a permissão do chamador para acessar recursos não autorizados.
- **Correção normativa:** definir envelope de execução (depth, fan-out, total calls, wall clock, tokens, custo, concurrency), propagação/redução de capabilities e cancelamento de toda a subárvore.
- **Critério de aceite sugerido:** testes de ciclo, fan-out e concorrência encerram dentro do envelope; cada nó tem principal/origem; cancelar o root cancela descendentes e libera leases/budget.
- **Dependências afetadas:** M9–M13; PR 160–169, 177–185, 218–231.

#### MA-002

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Domínio e políticas”; “UI e API”.
- **Evidência/omissão:** grupo tem moderator, routing/turn policy, max rounds, budget e synthesis mode, mas não há definição de terminação, empate, mensagem inválida, participante indisponível, ordem de resultados ou aceitação da síntese.
- **Risco:** grupos nunca terminam, sínteses descartam evidência ou uma resposta de agente é tratada como fato sem revisão.
- **Correção normativa:** definir state machine de grupo, quorum/termination rules, limites por rodada, ordem determinística, falhas parciais e status da síntese.
- **Critério de aceite sugerido:** cenários de consenso, timeout, agente ausente, erro e max rounds produzem estados finais observáveis e reproduzíveis.
- **Dependências afetadas:** M9; PR 155–172.

#### MA-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Workflow, scheduler e eventos”.
- **Evidência/omissão:** aprovação humana pode ser exigida, mas não se define quem aprova uma delegação, se a aprovação do root se propaga, como se mostra o contexto ao usuário ou o que acontece sem UI/TTY.
- **Risco:** execução autônoma fica bloqueada indefinidamente ou uma aprovação ampla autoriza ações novas e de maior risco.
- **Correção normativa:** vincular approval a operação/descendentes definidos, exigir reaprovação para escalada de capability e definir timeout, fallback e canal de aprovação por superfície.
- **Critério de aceite sugerido:** testes alteram parâmetros após aprovação, removem a UI e expiram a aprovação; nenhuma ação fora do envelope é executada.
- **Dependências afetadas:** M5, M9–M13, M15; PR 111, 161, 167, 184, 213–231, 248.

#### WF-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Workflow, scheduler e eventos”.
- **Evidência/omissão:** persistir estado, logs e recovery após crash é requisito, porém não se define estado de node/run, checkpoint, lease, retry, compensation, side effect idempotente ou como detectar execução interrompida.
- **Risco:** workflow reporta sucesso sem completar, executa pagamento/commit duas vezes ou permanece eternamente em `running`.
- **Correção normativa:** especificar state machine, durable checkpoint, idempotency key por node, lease/heartbeat, recovery policy, compensation e estados `unknown/needs_review` para side effects ambíguos.
- **Critério de aceite sugerido:** injeção de crash em cada transição recupera ou marca revisão sem duplicar side effect; restart e upgrade preservam run e logs.
- **Dependências afetadas:** M10–M11, M16; PR 176–188, 195–200, hardening.

#### WF-002

- **Severidade:** `MAJOR`
- **Natureza:** ambiguidade
- **Seção do source:** “Workflow, scheduler e eventos”.
- **Evidência/omissão:** workflow é “DAG persistente”, mas `Loop` é futuro e `SubWorkflow` é inicial; não há regra para subworkflow cíclico, reentrada, iteração limitada ou cycle detection no grafo persistido.
- **Risco:** um workflow aparentemente acíclico cria ciclo por subworkflow ou loop sem limite, contrariando a garantia de DAG.
- **Correção normativa:** declarar se Loop será node controlado fora do DAG ou extensão formal; aplicar validação de ciclos na publicação e limites de iteração/tempo/custo em runtime.
- **Critério de aceite sugerido:** grafo inválido é rejeitado antes de ativação; loops válidos têm limite obrigatório e trace por iteração; subworkflows não podem introduzir ciclo não declarado.
- **Dependências afetadas:** M10–M13; PR 173–185, 220–228.

#### WF-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Workflow, scheduler e eventos”.
- **Evidência/omissão:** scheduler cita interval, cron, one-shot, event/dependency triggers, missed-run, concorrência e histórico, mas não define timezone, DST, relógio, jitter, idempotência, política de catch-up ou limite de jobs.
- **Risco:** jobs rodam em horário inesperado, duplicam após suspensão do desktop ou disparam tempestades ao retornar online.
- **Correção normativa:** definir relógio UTC vs timezone do projeto, DST, missed-run (`skip/run-once/catch-up`), concurrency key, backoff, jitter, quota e comportamento offline.
- **Critério de aceite sugerido:** testes com DST, relógio alterado, sleep/resume, restart e múltiplas instâncias produzem agenda determinística e histórico correto.
- **Dependências afetadas:** M11; PR 191–203, M16.

#### WF-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Workflow, scheduler e eventos”.
- **Evidência/omissão:** Event Bus tem eventos extensíveis e triggers por evento, porém não define schema/versionamento, ordenação, replay, backpressure, autorização, retenção ou proteção contra trigger recursivo.
- **Risco:** evento duplicado/antigo inicia trabalho novo, consumer lento derruba runtime e um evento de `Finished` dispara recursivamente o mesmo workflow.
- **Correção normativa:** definir envelope de evento, event ID/causation/correlation, schema evolution, entrega, replay, DLQ/backpressure e limites de recursão.
- **Critério de aceite sugerido:** consumer idempotente processa duplicata; eventos incompatíveis vão para DLQ; replay não produz execução fora da autorização corrente.
- **Dependências afetadas:** M1, M9–M11, M13–M15; PR 023–024, 177, 188, 199, 218–219, 246–247.

#### WF-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** nodes `Approval`, `Parallel`, `Delay`, `SubWorkflow`.
- **Evidência/omissão:** nodes são nomeados, sem semântica de pause/resume após restart, cancelamento, timeout, branch failure, join incompleto, aprovação recusada ou mudança de versão do workflow durante um run.
- **Risco:** runs ficam órfãos ou unem resultados incompatíveis; cancelar um branch deixa locks e processos filhos.
- **Correção normativa:** definir contrato de entrada/saída e state machine de cada node, política de failure/join/cancel, version pin por run e retomada de approvals/delays.
- **Critério de aceite sugerido:** matriz de estados cobre sucesso, falha, timeout, cancelamento, restart e update; run mantém a versão publicada no início.
- **Dependências afetadas:** M10–M11; PR 182–190, 195–200.

### 3.6 API, CLI e UX

#### API-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “UI e API”.
- **Evidência/omissão:** APIs internas são apenas nomes (`projects.create`, `sessions.send`, etc.); não há request/response schema, erro, auth, versionamento, idempotência, pagination, streaming, cancellation ou capacidade exigida.
- **Risco:** UI, CLI, TUI e futuro remoto criam contratos incompatíveis e podem contornar autorização ou perder eventos de stream.
- **Correção normativa:** definir API tipada/versionada com envelopes, error codes, correlation/idempotency keys, auth context, capability checks e lifecycle de operações longas.
- **Critério de aceite sugerido:** cada método tem schema e contract test; UI e CLI usam o mesmo serviço; requests inválidos e não autorizados falham antes de side effects.
- **Dependências afetadas:** M1–M5, M9–M15; PR 029–031, 088–090, 155–172, 244–251.

#### API-002

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “UI e API”, CLI futuro.
- **Evidência/omissão:** comandos são listados, sem exit codes, output JSON/TTY, paginação, seleção de projeto, profiles, configuração, tratamento de secrets, cancelamento, prompts de aprovação ou modo CI.
- **Risco:** CLI não é automatizável por agentes nem scripts e pode expor segredos ou ficar esperando input indefinidamente.
- **Correção normativa:** especificar contrato CLI para modo interativo e não-interativo, saída estável, códigos, `--json`, `--project`, timeout, approval channel e redaction.
- **Critério de aceite sugerido:** smoke tests executam cada comando em TTY e CI sem input; output JSON é validado por schema e exit codes distinguem uso, policy, provider e runtime.
- **Dependências afetadas:** M0, M4–M5, M11–M15; PR 003, 089, 201, 204–217, 244–251.

#### API-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Workflow, scheduler e eventos”; “UI e API”.
- **Evidência/omissão:** streaming é dirigido por eventos e Tauri events, mas não há política de ordering, replay, reconnect, cursor, backpressure, finalização ou tradução entre evento interno e evento de superfície.
- **Risco:** UI perde tokens/status, reconecta em run errado ou mostra sucesso antes do commit persistido.
- **Correção normativa:** definir stream envelope, cursor/resume, terminal event, buffer, ack/backpressure e mapping autorizado por superfície.
- **Critério de aceite sugerido:** desconectar/reconectar durante stream retoma sem duplicação; um evento terminal só é emitido quando o estado durável permite a afirmação correspondente.
- **Dependências afetadas:** M4, M9–M11, M15; PR 085–091, 170–172, 246–247.

#### API-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Produto e objetivo”; “UI e API”.
- **Evidência/omissão:** o core será consumido por várias superfícies e remoto, porém não há política de compatibilidade semver, negociação de versão, depreciação ou capabilities.
- **Risco:** atualização do desktop, CLI, sidecar ou daemon quebra sessões e workflows persistentes.
- **Correção normativa:** definir versionamento de API/eventos/protocolos, janela de compatibilidade, capability negotiation e política de depreciação.
- **Critério de aceite sugerido:** matriz de versões aceita/rejeita combinações; fixtures de protocolos antigos passam no contract suite ou falham com erro acionável.
- **Dependências afetadas:** M0, M3–M4, M6, M14–M15, releases v0.1–v1.0.

#### UX-001

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “UI e API”.
- **Evidência/omissão:** a navegação e algumas telas são listadas, mas não há estados de loading/streaming/reconnect/offline/crash/recovery, conflito de edição, erro acionável, cancelamento ou execução em segundo plano.
- **Risco:** usuário não sabe se uma ação está rodando, duplicada, segura para repetir ou aguardando aprovação; operações destrutivas podem ser repetidas.
- **Correção normativa:** definir state machines de UX para sessão, tool approval, workflow, migration, update e provider; cada estado deve expor ação segura e evidência.
- **Critério de aceite sugerido:** testes E2E verificam sucesso, erro, timeout, cancelamento, restart e reconexão; uma ação em estado `unknown` não oferece retry cego.
- **Dependências afetadas:** M4–M5, M9–M11, M14–M16; PR 089–111, 170–172, 189–203, 237, 251.

#### UX-002

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “UI e API”; “Segurança e execução”.
- **Evidência/omissão:** Group Chat deve mostrar thinking, delegação, tool calls, custos, tokens e grafo, porém não há regra de redaction, minimização, role-based visibility ou confirmação contextual.
- **Risco:** UI vaza prompt, segredo, conteúdo de outro projeto ou dados de um agente privado; exibir “thinking” pode conflitar com privacidade e custo.
- **Correção normativa:** separar dados operacionais de conteúdo sensível; definir visibilidade por principal, redaction e consentimento para conteúdo de provider/agente.
- **Critério de aceite sugerido:** fixtures com tokens/PII não aparecem em telas ou exports não autorizados; cada campo visível tem regra de acesso testada.
- **Dependências afetadas:** M2, M4, M5, M9, M12; PR 053–055, 094–095, 109, 170–172, 208–217.

#### UX-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Domínio e políticas”; navegação por Files, Repositories, Memory e Settings.
- **Evidência/omissão:** não há UX normativa para arquivar/apagar projeto, editar memória, instalar skill, alterar permission policy, migrar dados ou restaurar backup.
- **Risco:** ação irreversível sem confirmação, memória editada sem provenance ou restore sobrescrevendo dados atuais.
- **Correção normativa:** definir operações reversíveis, confirmação com resumo e impacto, preview/dry-run, papel exigido, undo/backup e registro de auditoria.
- **Critério de aceite sugerido:** cada ação destrutiva tem confirmação contextual e caminho de recuperação documentado; restore nunca sobrescreve sem snapshot e consentimento.
- **Dependências afetadas:** M1, M7–M8, M16; PR 032, 133–134, 143–153, hardening.

#### UX-004

- **Severidade:** `MINOR`
- **Natureza:** requisito ausente
- **Seção do source:** “UI e API”.
- **Evidência/omissão:** não há requisitos de acessibilidade, teclado, contraste, i18n/l10n, formatos de data/horário, suporte a leitores ou densidade de informação.
- **Risco:** uso inconsistente em plataformas e dificuldade de operar aprovações/recuperações críticas.
- **Correção normativa:** incluir requisitos mínimos de acessibilidade e locale, com prioridade especial para approval, erro, progresso e dados de custo.
- **Critério de aceite sugerido:** checklist WCAG aplicável, navegação por teclado e testes de locale/horário na matriz de release.
- **Dependências afetadas:** M0, M4–M5, M9–M11; PR 003, 091–095, 170, 201, 203.

### 3.7 Observabilidade, auditoria e privacidade operacional

#### OBS-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Workflow, scheduler e eventos”; “Requisitos de qualidade”.
- **Evidência/omissão:** toda execução autônoma deve gerar trace com prompt assembly, request/response, tools, memórias, skills, erros, usage, custo e duração, mas não há classificação, redaction, retenção, acesso, criptografia ou consentimento.
- **Risco:** a regra de trace vira canal de exfiltração de prompts, tokens, arquivos e credenciais; também pode impedir uso em ambientes sensíveis.
- **Correção normativa:** separar auditoria mínima de conteúdo detalhado, definir campos sensíveis, redaction antes da persistência, TTL, encryption/access policy, opt-in/out permitido e export seguro.
- **Critério de aceite sugerido:** testes com segredos e PII verificam redaction em todos os sinks; acesso a trace exige capability; retention/delete são aplicados a traces e índices.
- **Dependências afetadas:** M4, M5, M7–M15, M16; PR 095, 120, 188, 202, 215, 247, hardening.

#### OBS-002

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Workflow, scheduler e eventos”; “Requisitos de qualidade”.
- **Evidência/omissão:** eventos são enumerados, sem envelope de correlação, métricas, níveis de log, health/readiness, error taxonomy, latência alvo, custo ou estado de provider/sandbox/worker.
- **Risco:** diagnóstico de falhas e de consumo é especulativo; o agente pode parecer travado ou saudável quando está sem provider, worker ou storage.
- **Correção normativa:** definir métricas e SLOs mínimos, correlation/causation IDs, health checks sem vazar segredos, log levels e diagnósticos por componente.
- **Critério de aceite sugerido:** um run pode ser seguido de UI até provider/tool/worker e volta; dashboards/exports mostram latência, retries, custo, bloqueios e estado de dependências.
- **Dependências afetadas:** M0, M3–M6, M9–M16; PR 023–024, 075, 120, 188, 202, 215, 247.

#### OBS-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Requisitos de qualidade”.
- **Evidência/omissão:** auditoria é citada somente indiretamente em traces/logs; não há audit log para login/provider, alteração de policy, aprovação, instalação, migration, update, delegation e uso de segredo.
- **Risco:** impossibilidade de atribuir ações, investigar incidente ou provar que uma alteração crítica foi autorizada.
- **Correção normativa:** definir audit events imutáveis ou tamper-evident, actor/source, before/after seguro, timestamp, retenção, consulta e controle de acesso.
- **Critério de aceite sugerido:** operações críticas geram evento auditável mesmo em erro; evento não contém segredo; export de auditoria permite correlacionar root e descendentes.
- **Dependências afetadas:** M1–M16, especialmente PR 035, 069, 111, 143–153, 207–217, 238–251.

#### OBS-004

- **Severidade:** `MINOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Requisitos de qualidade”; distribuição desktop.
- **Evidência/omissão:** não há mecanismo normativo de diagnóstico local, exportação redigida ou suporte offline para anexar evidências sem enviar conteúdo sensível.
- **Risco:** suporte pede cópia manual de logs ou o usuário desativa diagnóstico por medo de vazamento.
- **Correção normativa:** definir pacote de diagnóstico com allowlist de campos, redaction, consentimento, expiração e indicação de tamanho.
- **Critério de aceite sugerido:** export é reproduzível, redigido e recusável; fixture com secrets/PII não aparece no pacote.
- **Dependências afetadas:** M0, M4–M6, M16; PR 008, 120, 188, hardening.

### 3.8 Testes e gates de qualidade

#### TEST-001

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Requisitos de qualidade”.
- **Evidência/omissão:** a fonte enumera tipos de teste e gates, mas não define cobertura de comportamento, invariantes, targets de latência/custo, matriz de OS/provider/modelo, dados de fixture ou critério de bloqueio.
- **Risco:** “cargo test” verde pode não provar isolamento, recovery, segurança, compatibilidade ou comportamento de release.
- **Correção normativa:** converter cada requisito e invariante em cenários de aceite, suite responsável, ambiente, evidência e gate required/optional.
- **Critério de aceite sugerido:** existe matriz requisito→teste→artefato→gate por release; uma falha de security/architecture/migration bloqueia o card correspondente.
- **Dependências afetadas:** M0 e todas as milestones; PR 004–010, 020, 225, M16.

#### TEST-002

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Requisitos de qualidade”.
- **Evidência/omissão:** fuzz/security/load tests são listados sem threat cases, propriedades, boundaries ou definição de ambiente; não há testes de cross-project, escape, confused deputy, secret leakage e fail-closed.
- **Risco:** o gate de segurança se torna nominal e não detecta os riscos centrais do produto.
- **Correção normativa:** derivar threat model test plan com casos positivos/negativos para IPC, tools, filesystem, shell, Python, MCP, plugin, remote, secrets, prompt injection e migrations.
- **Critério de aceite sugerido:** cada ameaça de alta criticidade tem teste automatizado ou exceção formal aprovada; falhas de autorização/sandbox/leak bloqueiam release.
- **Dependências afetadas:** M2, M5–M6, M8–M9, M12–M15, M16; PR 044, 096–121, 136–154, 232–251.

#### TEST-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”; “Milestone 16”.
- **Evidência/omissão:** migrations, backups, release signing, updater e rollback são citados, mas não há matriz de upgrade/downgrade, falha de disco, corrupção, power loss, restore ou verificação de assinatura.
- **Risco:** perda de dados em release e rollback que não restaura uma instalação utilizável.
- **Correção normativa:** incluir testes destrutivos controlados de lifecycle de dados e release, com versões suportadas, artefatos e critérios de recuperação.
- **Critério de aceite sugerido:** cada release candidate passa upgrade, restart, rollback, restore e signature verification em Windows/Linux/macOS, com artifacts guardados.
- **Dependências afetadas:** M0, M1, M16; PR 016, 025–026, hardening de release.

#### TEST-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”; “Requisitos de qualidade”.
- **Evidência/omissão:** empacotamento multi-OS, sidecars, permissões e notificações são requisitos, sem matriz de versões de OS, arquitetura CPU, installer, keychain, shell, path, filesystem e disponibilidade de Python.
- **Risco:** produto funciona no ambiente de desenvolvimento e falha no primeiro run de uma plataforma suportada.
- **Correção normativa:** declarar matriz mínima de OS/arquitetura e testes de instalação, primeiro run, update, sidecar, permission prompt, uninstall e recovery.
- **Critério de aceite sugerido:** artifacts instaláveis e smoke tests por plataforma fazem parte do gate da release; capability indisponível é apresentada como tal.
- **Dependências afetadas:** M0, M5–M6, M11, M16; PR 002, 104, 113–121, 203, release.

#### TEST-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Runtime, providers e contexto”; “Requisitos de qualidade”; “Multi-Agent”.
- **Evidência/omissão:** MockProvider, testes de loop e compatibilidade são citados, sem fixtures de stream, erro, cancellation, partial output, retries, budget, cycle, fan-out, approval ou ordem de eventos.
- **Risco:** regressões no runtime aparecem apenas com providers reais ou loops caros e não determinísticos.
- **Correção normativa:** definir harness determinístico com relógio controlado, providers falsos, tool fakes e fault injection; cobrir invariantes de ordem, custo, limites e cancelamento.
- **Critério de aceite sugerido:** mesma fixture produz mesmo trace/state/usage; matriz cobre cada provider contract e cada terminal state do loop/workflow.
- **Dependências afetadas:** M3–M4, M9–M11; PR 056–060, 075–095, 160–188.

#### TEST-006

- **Severidade:** `MINOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Requisitos de qualidade”.
- **Evidência/omissão:** não há política para flakiness, isolamento de rede/clock/FS, quarantining, seed, repetição, limpeza de processos sidecar ou artefatos de falha.
- **Risco:** testes intermitentes são ignorados ou deixam workers/segredos no CI.
- **Correção normativa:** definir determinismo, seeds, teardown, timeout, retry controlado e política de quarantine com prazo.
- **Critério de aceite sugerido:** CI identifica e publica seed/logs/artifacts; nenhum teste é desativado sem issue, owner e data de retorno.
- **Dependências afetadas:** M0, M3–M6, M9–M16; PR 004–010, 020, 112–121, hardening.

### 3.9 Distribuição desktop, assinatura, atualização e rollback

#### DIST-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”.
- **Evidência/omissão:** empacotamento, assinatura e installer são reconhecidos como requisitos a fechar, sem política de identidade do publisher, cadeia de confiança, timestamp, certificados, key rotation, canais ou artefatos por OS/arquitetura.
- **Risco:** instaladores não confiáveis, impossibilidade de revogar chave ou publicar correção, e builds diferentes do que foi testado.
- **Correção normativa:** definir supply chain de build, signing/notarization por plataforma, proteção de chaves, provenance/SBOM, canais e verificação no install/update.
- **Critério de aceite sugerido:** cada artifact release é reprodutível/identificável, assinado por chave protegida e verificado em instalação limpa; falha de assinatura impede instalação.
- **Dependências afetadas:** M0, M16; PR 002, 004, 016 e hardening de release.

#### DIST-002

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”; “Releases alvo”.
- **Evidência/omissão:** atualização segura e rollback são citados, mas não há política de atomicidade, canal, pin, staged rollout, delta/full update, assinatura do manifest, compatibilidade de schema, interrupção de download ou fallback.
- **Risco:** update parcial, downgrade inseguro, rollback para binário incompatível com banco/skills/workflows ou rollout de artefato comprometido.
- **Correção normativa:** definir signed manifest, verificação antes de aplicar, A/B ou staging atômico, health check pós-update, rollback com backup e matriz de compatibilidade app↔schema↔sidecars.
- **Critério de aceite sugerido:** simulações de power loss, rede interrompida, assinatura inválida e health check falho deixam a versão anterior intacta e recuperável; rollback é comprovado no smoke test.
- **Dependências afetadas:** M1, M6, M10–M11, M14–M16; PR 016, 025–026, 119–121, 186–203, 232–251.

#### DIST-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Persistência e distribuição”.
- **Evidência/omissão:** sidecars, permissões, dados e deep links são mencionados, sem definir diretórios por OS, owner/permissions, upgrade de sidecar, processo órfão, uninstall, migração de secrets e dados.
- **Risco:** sidecar antigo permanece executando, arquivos ficam expostos ou uninstall remove referência necessária ao restore.
- **Correção normativa:** especificar layout/permissions, lifecycle de child processes, compatibilidade sidecar, uninstall/repair e sequência de migração de dados/secrets.
- **Critério de aceite sugerido:** instalação/upgrade/uninstall/repair não deixam processo ou segredo órfão; sidecar incompatível é recusado antes de aceitar requests.
- **Dependências afetadas:** M0, M6, M16; PR 002, 113–121, hardening.

#### DIST-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Persistência e distribuição”.
- **Evidência/omissão:** OAuth via deep link/callback e distribuição futura são citados, sem allowlist de schemes/hosts, validação de payload, associação ao request e comportamento quando outro app registra o scheme.
- **Risco:** callback é entregue ao aplicativo errado ou um payload malformado inicia ação privilegiada.
- **Correção normativa:** definir universal/app links quando disponíveis, state/PKCE, validação estrita e rejeição de callback não associado; deep link nunca deve executar tool diretamente.
- **Critério de aceite sugerido:** testes de scheme hijacking, payload inválido/replay e concorrência de login falham de forma segura; callback só completa o request correto.
- **Dependências afetadas:** M3, M14–M15; PR 070–071, 246–251.

### 3.10 Python, MCP, plugins e runtime remoto

#### EXT-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “Milestone 6 — Python Runtime”.
- **Evidência/omissão:** Python é sidecar JSON-RPC opcional com SDK, worker, lifecycle, permissões e logs, mas não há framing, schema/versionamento, handshake, timeout, cancelamento, restart, isolamento, limites ou política de dependências/instalação.
- **Risco:** worker travado, command injection, package supply-chain, processo órfão e bypass do Permission Engine.
- **Correção normativa:** especificar protocolo, capability negotiation, autenticação IPC, sandbox, quotas, venv/lockfile, allowlist de pacotes, lifecycle supervisionado e fail-closed.
- **Critério de aceite sugerido:** worker incompatível não conecta; crash/restart não duplica chamada; package sem pin/policy não instala; cada RPC passa pelo mesmo authorization/capability path.
- **Dependências afetadas:** M5–M6, M10, M14–M15; PR 103, 112–121, 180, 233–243, 248.

#### EXT-002

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “MCP e Plugins”.
- **Evidência/omissão:** “MCP Client primeiro” e transports stdio/HTTP/tool discovery são listados, sem versão de protocolo, handshake, origem, autenticação, TLS, pinning, schema validation, dynamic capability, prompt/resource access ou revogação.
- **Risco:** servidor MCP não confiável registra tool, recebe dados de projeto ou muda schema enquanto um run está aprovado.
- **Correção normativa:** definir MCP trust model, transport security, manifest/capabilities, approval por servidor/tool/version, validação de payload e lifecycle de conexão.
- **Critério de aceite sugerido:** tool desconhecida/incompatível é negada; alteração de schema/version exige reaprovação; testes cobrem servidor malicioso, HTTP sem TLS, disconnect e retry.
- **Dependências afetadas:** M5, M14–M15; PR 096–111, 232–237, 244–251.

#### EXT-003

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; “MCP e Plugins”.
- **Evidência/omissão:** plugins podem registrar providers, tools, memory backends, workflow nodes, connectors e event handlers, porém não há decisão in-process/out-of-process, ABI/API, isolamento, lifecycle, compatibilidade, assinatura ou permissions.
- **Risco:** plugin tem acesso ao processo e aos segredos do core; update de plugin corrompe banco/eventos ou quebra runtime.
- **Correção normativa:** definir tipos de plugin e níveis de isolamento; preferir protocolo versionado para código não confiável; exigir manifest, signature, capabilities, resource limits, install/update/disable e migration hooks seguros.
- **Critério de aceite sugerido:** plugin não autorizado não carrega; crash/timeout é isolado; versão incompatível não altera estado; capability e eventos são auditáveis.
- **Dependências afetadas:** M14–M16; PR 238–243, hardening.

#### EXT-004

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Produto e objetivo”; “Remote Runtime”.
- **Evidência/omissão:** há runtime transport, protocolo remoto, daemon autenticado, WebSocket, remote tools, project support e credential isolation como backlog, sem modelo de identidade, trust boundary, autorização, enrollment, rotação, replay, reconnect, offline e residência dos dados.
- **Risco:** daemon remoto vira uma extensão privilegiada do desktop; conexão perdida duplica trabalho; credenciais ou projetos cruzam nós.
- **Correção normativa:** especificar protocolo versionado com autenticação mútua quando aplicável, enrollment/revogação, autorização por projeto/tool, nonces/replay protection, leases, reconnect e policy de dados/credenciais.
- **Critério de aceite sugerido:** nó removido perde acesso imediatamente; replay é rejeitado; disconnect deixa runs em estado recuperável; testes provam isolamento de projetos e credenciais entre nós.
- **Dependências afetadas:** M15–M16; PR 244–251 e hardening.

#### EXT-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Segurança e execução”; Python/MCP/plugins/remote.
- **Evidência/omissão:** Python, MCP, plugins e remoto têm entradas próprias no inventário, mas não há capability vocabulary comum, error model, trace, budget, cancellation, approval e resource limits compartilhados.
- **Risco:** uma superfície nova bypassa controles existentes ou produz semântica diferente de tool nativa.
- **Correção normativa:** definir Extension/Tool Execution Contract único para authorization, lifecycle, timeout, cancellation, usage, audit e observability; cada adapter apenas traduz o transporte.
- **Critério de aceite sugerido:** contract suite comum passa para tool nativa, Python, MCP, plugin e remoto; todas as chamadas aparecem no mesmo grafo/traces/audit.
- **Dependências afetadas:** M5–M6, M14–M15; PR 096–121, 232–251.

#### EXT-006

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Produto e objetivo”; “Persistência e distribuição”.
- **Evidência/omissão:** Windows/Linux/macOS iniciais e Python opcional são suportados, sem política de runtime ausente, download de sidecar, compatibilidade de Python, proxy/rede ou execução offline.
- **Risco:** primeiro run baixa código sem consentimento, usa versão errada ou deixa produto inutilizável quando Python não está presente.
- **Correção normativa:** declarar o que é bundled/system/downloadable, hashes e assinatura, versão mínima, fallback sem Python e comportamento offline/restricted network.
- **Critério de aceite sugerido:** instalação sem Python executa features core; download é verificado e aprovado; cenário offline mostra capability indisponível sem travar core.
- **Dependências afetadas:** M0, M6, M14–M16; PR 002, 113–121, 233–251.

### 3.11 Skills e autoevolução

#### EVOL-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Memória, skills e evolução”.
- **Evidência/omissão:** skill inclui `SKILL.md`, scripts, templates, references e tests, e pode ser criada/testada/ativada, sem definir se scripts são executáveis, em qual sandbox, com quais dados/capabilities ou como conteúdo é separado de instrução confiável.
- **Risco:** skill importada ou gerada executa código, exfiltra segredos ou injeta instruções com os poderes do agente.
- **Correção normativa:** classificar cada arquivo e capability; executar scripts somente em sandbox mediado; validar manifest/schema; tratar conteúdo da skill como não confiável até promoção explícita.
- **Critério de aceite sugerido:** skill sem manifest/capabilities não ativa; scripts não alcançam fora do workspace autorizado; prompt injection em referência não altera security policy; instalação e execução são auditadas.
- **Dependências afetadas:** M5, M8, M13–M14; PR 136–154, 218–231, 238–243.

#### EVOL-002

- **Severidade:** `BLOCKER`
- **Natureza:** ambiguidade
- **Seção do source:** “Memória, skills e evolução”.
- **Evidência/omissão:** autonomy L0–L4 é definida por frases como “sugere”, “cria/testa”, “ativa após testes” e “altera ... dentro dos limites”, sem limites, aprovador, escopo, política de branch, evidência mínima ou ação proibida.
- **Risco:** L3/L4 pode ativar mudança de segurança, workflow ou configuração sem revisão humana ou pode ser interpretado de modo diferente por agentes.
- **Correção normativa:** transformar cada nível em capability matrix e gate: quem pode propor, modificar, testar, publicar, promover e reverter; exigir aprovação para security/permission/provider/release.
- **Critério de aceite sugerido:** cada transição de autonomia tem preconditions e approval; testes verificam que L0–L2 não ativam e que L3/L4 não ultrapassam escopo aprovado.
- **Dependências afetadas:** M8, M12–M13, M16; PR 149–154, 204–231.

#### EVOL-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Memória, skills e evolução”; regra imutável 8.
- **Evidência/omissão:** versionamento e rollback de skills são mencionados, mas não há pin por session/workflow, atomic activation, compatibilidade de schema, migrations, rollout, canary ou rollback após runs em andamento.
- **Risco:** uma skill nova muda resultados de sessões existentes, corrompe workflow ou torna rollback incompleto.
- **Correção normativa:** versionar/pinar skill no contexto; ativação deve ser atômica, compatível e observável; definir rollout e política para runs já iniciados.
- **Critério de aceite sugerido:** run registra a versão da skill e continua com ela; rollback restaura versão e metadata sem apagar histórico; incompatibilidade impede ativação.
- **Dependências afetadas:** M8, M10, M13; PR 143–154, 176–188, 221–228.

#### EVOL-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Memória, skills e evolução”.
- **Evidência/omissão:** evaluator gera candidate, testa, avalia e publica conforme policy, sem dataset, baseline, métricas, determinismo, tolerância, proteção contra reward hacking/poisoning ou evidência persistida.
- **Risco:** autoevolução otimiza métrica errada, aprende comportamento inseguro ou ativa regressão difícil de atribuir.
- **Correção normativa:** definir benchmark versionado, baseline, métricas funcionais e de segurança, confidence/threshold, holdout, revisão e rollback automático com causa.
- **Critério de aceite sugerido:** candidate só promove se superar baseline sem regressão de security/quality; avaliação é reproduzível por seed e artifacts; rollout é interrompido em alerta.
- **Dependências afetadas:** M8, M12–M13, M16; PR 146–154, 220–228, security/release hardening.

#### EVOL-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Domínio e políticas”; “Memória, skills e evolução”.
- **Evidência/omissão:** skills globais entram por importação explícita, mas não há origem, pin, ownership, dependências transitivas, conflito de nomes, precedence ou remoção.
- **Risco:** skill global contorna isolamento ou introduz instruções/tools em projeto sem revisão local.
- **Correção normativa:** import deve criar referência imutável por projeto, com provenance, version pin, review/capability approval e namespace; remoção não deve apagar histórico de runs.
- **Critério de aceite sugerido:** projeto sem import não enxerga skill global; import é auditado e reproduzível; conflito/version mismatch é rejeitado ou resolvido por regra documentada.
- **Dependências afetadas:** M7–M8, M13–M14; PR 140–143, 148–154, 238–243.

#### EVOL-006

- **Severidade:** `MINOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Memória, skills e evolução”.
- **Evidência/omissão:** estados draft/testing/active/deprecated/archived/blocked são enumerados, mas não há transições válidas, actor, reason, timestamps ou retenção.
- **Risco:** estado exibido na UI não corresponde ao comportamento do loader e não há explicação para bloqueio/depreciação.
- **Correção normativa:** definir state machine e metadata obrigatória de lifecycle, incluindo causa, actor/policy e versão substituta.
- **Critério de aceite sugerido:** transição inválida é rejeitada; cada mudança gera evento/audit e o loader respeita o estado publicado.
- **Dependências afetadas:** M8, M13–M14; PR 138–154, 238–240.

### 3.12 Desenvolvimento por agentes, PR queue e roadmap

#### GOV-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Inventário original de PRs”; “Entregável esperado do planejamento”.
- **Evidência/omissão:** há títulos numerados de PRs, mas não há, para cada card, objetivo, escopo/out, dependências, arquivos, acceptance, testes, segurança, migração, docs, rollback, riscos, owner ou artifact esperado.
- **Risco:** não é possível selecionar “exatamente um card com dependências satisfeitas” nem revisar se uma mudança está dentro do escopo.
- **Correção normativa:** expandir cada PR em ficha canônica e tornar campos obrigatórios; não considerar título de PR como especificação.
- **Critério de aceite sugerido:** validação automática rejeita card incompleto; um agente consegue executar o card sem inventar contrato; reviewer consegue verificar acceptance e rollback.
- **Dependências afetadas:** PR 001–251; todo o contrato de execução por agentes.

#### GOV-002

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Inventário original de PRs”; “Milestone 16”.
- **Evidência/omissão:** M16 é explicitamente não numerada e deve virar PRs adicionais, enquanto o entregável exige queue executável, DAG e PR #001 exata. Não há lista desses cards nem decisão sobre a sequência inicial.
- **Risco:** hardening essencial fica sem owner/dependência e a execução começa em um PR arbitrário; o requisito de “PR #001 exata” não é verificável.
- **Correção normativa:** decompor M16 em cards numerados com dependências e definir a ficha completa do PR #001, incluindo seu non-goal e acceptance.
- **Critério de aceite sugerido:** existe uma fila única, sem IDs duplicados, cobrindo M16; o card #001 é único, apontado por nome/arquivo e validado contra gates.
- **Dependências afetadas:** M0, M16; releases v0.1–v1.0.

#### GOV-003

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Desenvolvimento Agents” (M12); “Regras imutáveis”, item 15.
- **Evidência/omissão:** há coding/reviewer/QA/security/architecture/release agents e regra Git + testes, mas não há contrato operacional para branch/worktree, base SHA, estado dirty, ownership de arquivos, comandos autorizados, secrets, review independente ou concorrência.
- **Risco:** agentes alteram `main`, sobrescrevem trabalho paralelo, executam comandos fora do escopo ou aprovam a própria mudança sem evidência.
- **Correção normativa:** definir PR Execution Contract: preflight, branch/worktree, scope, comandos, artefatos, gates, handoff, review, rollback e critérios de parada/blocker.
- **Critério de aceite sugerido:** um run de agente deixa registro de branch/base SHA/status, arquivos alterados e comandos/resultados; mudanças fora do escopo falham no gate; comentário de IA não é aprovação.
- **Dependências afetadas:** M0, M12–M13, M16; PR 017, 204–231 e toda a queue.

#### GOV-004

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Entregável esperado do planejamento”.
- **Evidência/omissão:** exige SDD mestre sem perdas, Architecture Invariants, política automatizada e demos verificáveis, mas não há matriz de rastreabilidade entre requisito, ADR, PR, teste e demo.
- **Risco:** requisitos desaparecem entre milestones e um PR “verde” implementa apenas parte do SDD.
- **Correção normativa:** criar requirement IDs estáveis e matriz `requirement → invariant/ADR → PR → test/artifact → release demo`; mudanças devem atualizar a matriz.
- **Critério de aceite sugerido:** nenhuma regra imutável ou requisito de segurança fica sem owner e teste; revisão de PR identifica cobertura e não cobertos.
- **Dependências afetadas:** todos os documentos e PR 001–251.

#### GOV-005

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Workspace conceitual”; inventário de adapters e extensões.
- **Evidência/omissão:** crates, providers, SQLx, Tauri, Python, MCP, Docker/Podman/SSH/WASM e plugins são propostos, sem justificativa de necessidade, manutenção, licença, segurança, custo e custo de substituição.
- **Risco:** dependências incompatíveis, superfície de supply chain excessiva e decisões irreversíveis sem avaliação.
- **Correção normativa:** cada dependência/action nova deve ter ficha de decisão com alternativa considerada, licença, manutenção, risco, custo e plano de remoção/substituição.
- **Critério de aceite sugerido:** CI e revisão recusam dependência sem ficha e sem lock/hash; SBOM/licenças entram no artifact de release.
- **Dependências afetadas:** M0, M3, M6, M10, M14–M16; PR 001–003, 061–066, 112–121, 232–251.

#### GOV-006

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Requisitos de qualidade”; “Releases alvo”.
- **Evidência/omissão:** gates gerais (`fmt`, Clippy, testes, lint, security, architecture) são nomeados, sem dizer quais são required por PR/release, como artifacts são publicados ou como falha assíncrona/rebase é revalidada.
- **Risco:** merge de PR sem gate aplicável ou agente reportando estado obsoleto após rebase/CI tardio.
- **Correção normativa:** definir pipeline por tipo de card e release, required checks, artifact retention, invalidation após rebase e regra de parada para CI/security falho.
- **Critério de aceite sugerido:** cada card declara gates; um SHA novo invalida resultados antigos; release não promove com check required ausente/falho.
- **Dependências afetadas:** M0, M16; PR 004–016, 215–217 e queue inteira.

#### GOV-007

- **Severidade:** `MINOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Entregável esperado do planejamento”.
- **Evidência/omissão:** ondas paralelas e caminho crítico são exigidos, mas não há critério de independência, ownership ou política para conflito de arquivos entre agentes.
- **Risco:** paralelismo teórico causa merge conflict, trabalho duplicado ou dependência implícita.
- **Correção normativa:** incluir wave ID, owner, locks de arquivo/contrato e condições de paralelismo; itens que compartilham schema/evento devem ser sequenciais.
- **Critério de aceite sugerido:** grafo identifica caminho crítico e waves; cada par paralelo é verificável como independente ou explicitamente serializado.
- **Dependências afetadas:** PR 001–251, especialmente M0–M3 e M9–M15.

### 3.13 Roadmap, releases e governança de dados

#### ROAD-001

- **Severidade:** `BLOCKER`
- **Natureza:** requisito ausente
- **Seção do source:** “Releases alvo”; “Entregável esperado do planejamento”.
- **Evidência/omissão:** releases são apenas associações de milestones (`v0.1 Foundation`, ..., `v1.0 Production`), sem demo, acceptance, non-goals, critérios de segurança, dados, compatibilidade ou decisão de “ship/no-ship”.
- **Risco:** milestone concluída sem produto demonstrável, ou v1.0 anunciada sem update/recovery/signing provados.
- **Correção normativa:** definir release contract com demo verificável, gates required, matriz suportada, riscos aceitos, rollback e evidência mínima.
- **Critério de aceite sugerido:** cada release tem checklist assinado por testes, segurança, arquitetura e distribuição; a demo usa artifact instalado e dados de teste reproduzíveis.
- **Dependências afetadas:** releases v0.1–v1.0; PR 001–251 e M16.

#### ROAD-002

- **Severidade:** `MAJOR`
- **Natureza:** ambiguidade
- **Seção do source:** “Persistência e distribuição”; “Milestone 16”; “Releases alvo”.
- **Evidência/omissão:** backups, rate/resource limiting, audit, security, recovery, signing e updater aparecem em hardening/v1.0, mas isolamento, secrets, autorização, budgets, recovery e audit são necessários desde as primeiras features que executam tools e persistem dados.
- **Risco:** releases intermediárias operacionais acumulam dados e capabilities sem controles básicos, tornando migração/hardening posterior inseguro.
- **Correção normativa:** separar controles mínimos obrigatórios por milestone de hardening avançado; não permitir tool, provider credential ou workflow persistente sem baseline de autorização, secrets, limits e recovery.
- **Critério de aceite sugerido:** matriz “control → primeira release obrigatória → owner → teste” não deixa controles de alta criticidade somente para M16.
- **Dependências afetadas:** M1–M6 e M10–M16; PR 044, 068–069, 099, 110–111, 186–188 e hardening.

#### ROAD-003

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Releases alvo”; “Persistência e distribuição”; “MCP e Plugins”.
- **Evidência/omissão:** não há política de compatibilidade e suporte para DB, API/eventos, skills/workflows, providers, plugins, sidecars ou protocolo remoto entre v0.x e v1.0.
- **Risco:** usuários não sabem se update preserva dados e comportamento; rollback de skill/provider/plugin pode ser impossível.
- **Correção normativa:** definir matriz de compatibilidade, janela de suporte, depreciação, migration path e política de breaking change por artifact.
- **Critério de aceite sugerido:** notas de release e testes informam combinações suportadas; update rejeita incompatibilidade antes de alterar dados.
- **Dependências afetadas:** M1, M3–M4, M6, M8, M10, M14–M16.

#### COM-001

- **Severidade:** `MAJOR`
- **Natureza:** requisito ausente
- **Seção do source:** “Produto e objetivo”; “Runtime, providers e contexto”; “Persistência e distribuição”.
- **Evidência/omissão:** conteúdo de projeto, prompts, memórias, arquivos, traces e usage pode ir para múltiplos providers, embeddings, MCP, plugins e nós remotos, sem política de consentimento, residência, retenção, transferência, export/delete ou termos.
- **Risco:** exposição de dados sensíveis, conflito regulatório/contratual e impossibilidade de informar o usuário onde o conteúdo foi processado.
- **Correção normativa:** definir classificação e fluxo de dados, provider data policy, consentimento e configuração por projeto; documentar residency/retention e impedir envio não autorizado.
- **Critério de aceite sugerido:** UI/API mostram destino e policy antes de conectar provider/remote; testes bloqueiam provider incompatível com a policy do projeto; delete/export percorre todos os stores permitidos.
- **Dependências afetadas:** M3–M4, M7, M14–M15, M16; PR 068–076, 122–135, 232–251.

#### COM-002

- **Severidade:** `SUGGESTION`
- **Natureza:** requisito ausente
- **Seção do source:** documento inteiro.
- **Evidência/omissão:** o inventário usa muitos termos carregados (“core”, “runtime”, “memory”, “skill”, “provider”, “workflow”, “agent”, “remote”) sem glossário canônico nem registro de decisões abertas.
- **Risco:** agentes e revisores atribuem significados diferentes a conceitos iguais e espalham ambiguidades para PRs.
- **Correção normativa:** adicionar glossário, decision log e lista de perguntas abertas com owner e data de decisão; ligar termos aos schemas/ADRs.
- **Critério de aceite sugerido:** cada termo usado em um card aponta para definição única; decisões fechadas removem alternativas conflitantes do SDD mestre.
- **Dependências afetadas:** todos os documentos e PRs.

## 4. Conjunto mínimo de decisões para sair de “Not ready”

As decisões abaixo são o mínimo; não são implementação nem aprovação de qualquer PR:

1. **Fronteira canônica:** Tauri como shell/adaptador, Agent Core independente, direção de dependências e modelo de processos/trust boundaries.
2. **Escopo de release:** superfícies realmente suportadas em cada versão, não-goals, capabilities mínimas e demos verificáveis.
3. **Identidade e autorização:** principals, project isolation, capability model, propagação/redução em delegações, default deny e aprovação humana.
4. **Sandbox e extensões:** perfis por OS, limites de filesystem/rede/processo, política de indisponibilidade e modelo comum para tools, Python, MCP, plugins e remoto.
5. **Contratos:** schemas versionados para API, eventos, providers, tool calls, approvals, streams, errors, cancellation e usage/budget.
6. **Durabilidade:** schema lógico, invariantes, transações/outbox, idempotência, recovery, concorrência SQLite, blob lifecycle e retention.
7. **Migração e dados:** forward/downgrade policy, backup/restore, encryption, secret migration, compatibilidade por release e teste de power loss.
8. **Autonomia e autoevolução:** capability matrix L0–L4, gates, revisão, benchmark, rollout, pin por run e rollback seguro.
9. **Distribuição:** matriz OS/arquitetura, signing/notarization, provenance/SBOM, canais, updater atômico, rollback e deep-link seguro.
10. **Observabilidade e privacidade:** redaction, retenção, auditoria, correlation IDs, métricas/SLOs, consentimento e export diagnóstico.
11. **Testing/gates:** threat-driven test plan, contract/E2E/recovery/release matrix, required checks e evidências por release.
12. **Execução por agentes:** PR Execution Contract, fichas completas dos cards, dependências DAG, waves, caminho crítico, ownership, branch/worktree e regra de blocker.
13. **Roadmap completo:** decomposição numerada de M16 e definição exata do PR #001, com rastreabilidade requisito→ADR→PR→teste→demo.

Após essas decisões, o SDD mestre deve incorporar as correções, remover a contradição `ARCH-001`, converter os achados em requisitos rastreáveis e só então selecionar um card com dependências comprovadamente satisfeitas. Até lá, nenhuma afirmação de implementação/executado deve ser feita; esta revisão registra apenas lacunas e critérios de aceite propostos.
