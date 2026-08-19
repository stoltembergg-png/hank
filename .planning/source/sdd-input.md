# SDD Input Pack — Plataforma Desktop Multiagente

Status da fonte: Draft v0.1 fornecido pelo usuário em 2026-08-17.
Uso: fonte de planejamento; não é implementação e não constitui prova de comportamento.
Regra operacional: não inicializar Git, não criar código, não misturar este trabalho com o worktree hospedeiro do Orca.

## Produto e objetivo

Plataforma desktop multiagente com Rust + Tauri 2 como núcleo, Python opcional como extensão, inicialmente Windows/Linux/macOS. O produto opera projetos isolados contendo agentes, grupos, sessões, memória, skills, ferramentas, workflows, automações, tarefas, arquivos, repositórios, histórico, permissões, orçamento e contexto compartilhado. O core deve ser reutilizável por Desktop, CLI, TUI, Web, Mobile, API e workers remotos.

## Arquitetura normativa proposta

Tauri UI -> Application API -> Agent Runtime. O runtime compõe Model Gateway, Tool Runtime, Workflow Engine, Memory Engine, Skill Engine, Scheduler, Project Manager e Event Bus. Opção escolhida: Core Rust modular + Tauri; microsserviços locais são evolução posterior, não v1.

Workspace conceitual: apps/desktop, apps/cli; crates agent-core, agent-runtime, agent-orchestrator, agent-protocol, project-core, session-core, memory-core, skill-core, workflow-core, scheduler-core, tool-core, provider-core e adapters de OpenAI/Anthropic/Google/OpenRouter/Ollama, auth-core, storage-core, secrets-core, sandbox-core, event-bus, telemetry-core; python/runtime, tools, sdk; skills, migrations, docs, tests.

## Domínio e políticas

Agent persistente: identity, role, personality, instructions, model/provider policy, tool/memory/skill/context policy, autonomy, budget e project bindings. ResponseProfile estruturado, hierarquia determinística de instruções: system -> security -> project -> agent -> workflow -> skill -> conversation -> user.

Project é unidade de isolamento e pode ter agentes, grupos, sessões, memórias, skills, workflows, tasks, repositories, folders, artifacts, permissions, settings e arquivos PROJECT.md/AGENTS.md/MEMORY.md/SOUL.md. Compartilhamento entre projetos é proibido por padrão; skills globais só entram por importação explícita.

Session contém project, agent, participantes, mensagens, tool calls, artifacts, snapshots, tokens, custos e traces. Streaming é dirigido por eventos internos. AgentGroup tem members, moderator, routing/turn policy, max rounds, budget, shared context e permissions. Delegações devem validar existência, acessibilidade, permissão, orçamento, profundidade e ciclos via InvocationGraph.

## Runtime, providers e contexto

Agent Orchestrator realiza context assembly -> seleção de modelo -> chamada -> tool request -> execução -> próximo turno, sem conhecer providers concretos. ModelProvider normaliza stream/complete, modelos, capabilities e custo. Providers iniciais: OpenAI, Anthropic, Gemini, OpenRouter, Ollama e OpenAI-compatible; fallback para 429, timeout, outage e quota.

Context Builder escolhe system/security/project/agent/skills/memories/conversation/task/tools/group context com orçamento configurável; não envia todo banco de memória. Compressão cria checkpoints e arquiva mensagens brutas sem destruição automática. Model routing por complexidade e modalidade. Budget por projeto/agente/workflow/task com limites de tokens e custo.

## Segurança e execução

SecretsService armazena credenciais em OS keychain ou Tauri Stronghold; nunca plaintext em SQLite, .env, frontend localStorage ou logs. OAuth via browser/deep link/callback/token exchange. Tool calls passam por permission engine e sandbox; LLM nunca chama shell diretamente. Trusted/restricted/isolated; futuras opções Docker/Podman/SSH/remote/WASM. Confirmações always_allow/ask_once/ask_every_time/deny; ações destrutivas, force push, credentials, pagamentos e instalação de pacotes podem exigir aprovação humana.

Tool comum possui name, schema, permissions, timeout, execution environment e handler. Ferramentas iniciais filesystem, terminal, git, http/web/browser/search/python/process/clipboard/notifications. Python é sidecar opcional via JSON-RPC, com worker, SDK, lifecycle, permissões e logs; Python nunca é requisito do Agent Core. MCP Client primeiro e MCP Server depois; plugins podem registrar providers/tools/memory backends/workflow nodes/connectors/event handlers.

## Memória, skills e evolução

Working, Short-Term e Long-Term Memory; tipos fact, preference, decision, lesson, project_context, technical_context, failure e successful_pattern. Pipeline conversation -> candidate extractor -> importance -> dedupe -> storage -> retrieval index. O modelo sugere, mas não grava diretamente; deve haver edição e isolamento.

Skill reutilizável com SKILL.md, manifest, scripts, templates, references, tests e metadata; estados draft/testing/active/deprecated/archived/blocked; versões preservadas e rollback. Learning evaluator gera candidate, testa, avalia e só publica conforme política. Autonomia L0 sem evolução, L1 sugere, L2 cria/testa, L3 ativa após testes, L4 altera skills/workflows/config dentro dos limites. Runtime Rust só muda via Git/branch/worktree/testes/PR/review/policy/release.

## Workflow, scheduler e eventos

Workflow é DAG persistente, não prompt gigante. Nodes iniciais Agent, Tool, Python, Condition, Parallel, Delay, Approval e SubWorkflow; futuramente Webhook, HTTP, GitHub, Scheduler, Loop e HumanInput. Deve persistir estado, logs, recovery após crash e sobreviver a restart. Scheduler suporta one-shot, interval, cron, event e dependency triggers, missed-run policy, concorrência, histórico e notificações.

Event Bus: AgentCreated/Started/Finished, MessageReceived/Generated, ToolStarted/Finished, WorkflowStarted/Finished, SkillCreated/Updated, ProjectChanged, ProviderConnected, TaskCreated/Completed e extensíveis. Toda execução autônoma gera trace com prompt assembly, provider request/response, tools, delegações, memory/skills, erros, usage, custo e duração.

## Persistência e distribuição

SQLite inicialmente com SQLx/Tokio e migrations. Entidades: projects, agents, groups/members, sessions/messages, memories/embeddings, skills/versions, workflows/nodes/edges/runs, tasks/runs, providers/accounts, scheduled_jobs, tool_calls, artifacts e usage_events. Blobs grandes ficam em data/artifacts/sessions/skills/projects/cache/logs, SQLite guarda metadados/referências.

Desktop precisa contemplar empacotamento Windows/Linux/macOS, assinatura, instalador, permissões, sidecars, migração de dados, backup/restore, atualização segura, rollback de release, deep links e distribuição futura. Estes pontos são requisitos a fechar, não presumidos como resolvidos pela escolha Tauri 2.

## UI e API

Navegação global Projects, Chats, Agents, Groups, Tasks, Workflows, Skills, Automations, Models, Usage e Settings. Dentro de projeto: Chats, Agents, Groups, Tasks, Workflows, Skills, Files, Repositories, Memory e Settings. Agent Builder sem exigir YAML. Group Chat mostra participantes, thinking, delegação, tool calls, custos, tempo, tokens e grafo. UI chama serviços/comandos; nenhuma tela acessa SQLite.

CLI futuro: agent project list, chat, run, workflow run, skill list, provider list. API interna conceitual projects.create, agents.create, sessions.send, groups.send, workflows.run, skills.install e providers.connect.

## Requisitos de qualidade

Cada crate deve ter unit/integration/contract tests. Também: MockProvider determinístico, provider mocks, workflow deterministic tests, permissions, migrations, skills, multi-agent loop, crash recovery, provider compatibility, security/fuzz/load/release tests. Gates: fmt, clippy, cargo test, frontend lint/typecheck/tests, security/dependency, architecture rules, ADR para mudanças arquiteturais. DoD: problem, scope, non-goals, implementation, tests, security/migration/docs impact, rollback; sem refatoração alheia, TODO oculto, testes desativados, unwrap inseguro, plaintext secrets, UI->DB direto ou provider logic no core.

## Regras imutáveis da fonte

1. Tauri não é Agent Core.
2. Frontend nunca acessa SQLite diretamente.
3. Agent Core não depende de providers concretos.
4. Providers dependem de provider-core.
5. Tool calls sempre passam pelo Permission Engine.
6. Shell irrestrito não é default.
7. Skills não alteram runtime silenciosamente.
8. Autoevolução versionada e reversível.
9. Projetos isolados por padrão.
10. Invocações têm limite de profundidade/ciclos.
11. Workflows persistentes sobrevivem a reinicialização.
12. Segredos nunca ficam plaintext.
13. Python não é requisito do core.
14. Atividade autônoma tem trace.
15. Mudança no app passa por Git + testes.

## Inventário original de PRs fornecido pelo usuário

### Milestone 0 — Engineering Foundation (PR 001–020)
001 Initialize Cargo workspace; 002 Initialize Tauri 2 desktop; 003 Add frontend workspace; 004 Create CI build workflow; 005 Add Rust fmt checks; 006 Add Rust clippy checks; 007 Add Rust unit test workflow; 008 Add frontend lint; 009 Add frontend typecheck; 010 Add frontend tests; 011 Add Dependabot; 012 Add CodeQL; 013 Add conventional commits; 014 Add PR title validation; 015 Add changelog automation; 016 Add release workflow; 017 Define repository contribution rules; 018 Add architecture doc; 019 Add ADR structure; 020 Add test fixtures framework.

### Milestone 1 — Core Domain (PR 021–038)
021 Introduce typed IDs; 022 Add domain error model; 023 Add application event model; 024 Implement event bus; 025 Add SQLite; 026 Add SQL migrations; 027 Add project entity; 028 Add project repository; 029 Add create project service; 030 Add list project service; 031 Add update project service; 032 Add archive project service; 033 Add folders to projects; 034 Add repositories to projects; 035 Add project settings; 036 Add project UI listing; 037 Add create-project UI; 038 Add project detail UI.

### Milestone 2 — Agent Domain (PR 039–055)
039 Add Agent entity; 040 Add Agent repository; 041 Add Agent configuration schema; 042 Add personality schema; 043 Add instruction hierarchy; 044 Add tool permission schema; 045 Add model policy schema; 046 Add autonomy policy; 047 Add budget policy; 048 Add agent CRUD services; 049 Add agent list UI; 050 Add Agent Builder identity page; 051 Add personality page; 052 Add model page; 053 Add permissions page; 054 Add instructions page; 055 Add autonomy page.

### Milestone 3 — Provider System (PR 056–077)
056 Define ModelProvider trait; 057 Define model capability schema; 058 Define normalized request; 059 Define normalized response; 060 Define streaming events; 061 Implement OpenAI-compatible adapter; 062 Add OpenAI provider; 063 Add Anthropic provider; 064 Add Gemini provider; 065 Add OpenRouter provider; 066 Add Ollama provider; 067 Implement provider registry; 068 Add credential service; 069 Add encrypted secret storage; 070 Add OAuth framework; 071 Add OAuth callback handling; 072 Add provider settings UI; 073 Add model discovery; 074 Add model selector; 075 Add provider health check; 076 Add fallback policy.

### Milestone 4 — Chat Runtime (PR 078–095)
078 Add Session entity; 079 Add Message entity; 080 Add session storage; 081 Add message storage; 082 Add context builder interface; 083 Add basic context builder; 084 Add agent execution state machine; 085 Add provider streaming; 086 Add cancellation; 087 Add retry policy; 088 Add session service; 089 Add chat command; 090 Add streaming Tauri events; 091 Add chat UI; 092 Add markdown rendering; 093 Add code block rendering; 094 Add model/provider indicators; 095 Add token metrics.

### Milestone 5 — Tools (PR 096–111)
096 Define Tool trait; 097 Define tool schema; 098 Add tool registry; 099 Add permission evaluator; 100 Add filesystem read tool; 101 Add filesystem write tool; 102 Add directory listing; 103 Add process execution primitive; 104 Add terminal tool; 105 Add HTTP tool; 106 Add Git status tool; 107 Add Git diff tool; 108 Add Git commit tool; 109 Add tool-call rendering; 110 Add timeout handling; 111 Add confirmation policies.

### Milestone 6 — Python Runtime (PR 112–121)
112 Define worker protocol; 113 Create Python worker; 114 Add JSON-RPC transport; 115 Add Python process lifecycle; 116 Add Python SDK; 117 Add Python tool registration; 118 Add Python execution tool; 119 Add dependency environment management; 120 Add Python logs; 121 Add Python permissions.

### Milestone 7 — Memory (PR 122–135)
122 Add memory entity; 123 Add memory repository; 124 Add memory type taxonomy; 125 Add memory candidate extractor; 126 Add memory importance scoring; 127 Add deduplication; 128 Add keyword retrieval; 129 Add embedding interface; 130 Add vector retrieval backend; 131 Add context memory selector; 132 Add memory UI; 133 Add manual memory editing; 134 Add project memory isolation; 135 Add agent memory policies.

### Milestone 8 — Skills (PR 136–154)
136 Define skill manifest; 137 Add skill parser; 138 Add skill repository; 139 Add skill loader; 140 Add project skills; 141 Add global skills; 142 Add agent skill bindings; 143 Add skill versioning; 144 Add skill UI; 145 Add skill editor; 146 Add skill test framework; 147 Add skill validation; 148 Add skill creation tool; 149 Add learning evaluator; 150 Add skill candidate generation; 151 Add autonomous skill test; 152 Add skill activation policies; 153 Add skill rollback; 154 Add skill lifecycle curator.

### Milestone 9 — Multi-Agent (PR 155–172)
155 Add AgentGroup entity; 156 Add group repository; 157 Add group membership; 158 Add group session; 159 Add mention parser; 160 Add agent invocation protocol; 161 Add delegation tool; 162 Add invocation graph; 163 Add cycle detection; 164 Add maximum delegation depth; 165 Add parallel invocation; 166 Add group budgets; 167 Add moderator policy; 168 Add round policy; 169 Add synthesis mode; 170 Add group chat UI; 171 Render agent-to-agent messages; 172 Render delegation graph.

### Milestone 10 — Workflow Engine (PR 173–190)
173 Define workflow entity; 174 Define workflow node; 175 Define workflow edge; 176 Add workflow persistence; 177 Add execution engine; 178 Add AgentNode; 179 Add ToolNode; 180 Add PythonNode; 181 Add ConditionNode; 182 Add ParallelNode; 183 Add DelayNode; 184 Add ApprovalNode; 185 Add SubWorkflowNode; 186 Add workflow state persistence; 187 Add crash recovery; 188 Add workflow logs; 189 Add workflow editor; 190 Add workflow run viewer.

### Milestone 11 — Scheduler (PR 191–203)
191 Add scheduled job entity; 192 Add interval scheduling; 193 Add cron parsing; 194 Add one-shot scheduling; 195 Add scheduler persistence; 196 Add scheduler worker; 197 Add missed-run policy; 198 Add concurrent-run protection; 199 Add workflow scheduler integration; 200 Add agent scheduler integration; 201 Add automation UI; 202 Add execution history; 203 Add desktop notifications.

### Milestone 12 — Development Agents (PR 204–217)
204 Add repository workspace manager; 205 Add Git worktree manager; 206 Add branch policy; 207 Add task-to-branch mapping; 208 Add coding agent profile; 209 Add reviewer agent profile; 210 Add QA agent profile; 211 Add security agent profile; 212 Add architecture agent profile; 213 Add PR generation workflow; 214 Add review workflow; 215 Add CI status integration; 216 Add fix-review workflow; 217 Add release-agent workflow.

### Milestone 13 — Autonomous Evolution (PR 218–231)
218 Add improvement observation event; 219 Add improvement candidate entity; 220 Add self-evaluation workflow; 221 Add skill improvement proposal; 222 Add workflow improvement proposal; 223 Add agent configuration proposal; 224 Add automated evaluation; 225 Add regression evaluation; 226 Add improvement scoring; 227 Add automatic skill rollout; 228 Add automatic rollback; 229 Add self-development issue creation; 230 Add self-development branch creation; 231 Add self-development PR creation.

### Milestone 14 — MCP and Plugins (PR 232–243)
232 Add MCP transport abstraction; 233 Add MCP stdio client; 234 Add MCP HTTP client; 235 Add MCP tool discovery; 236 Add MCP permission integration; 237 Add MCP settings UI; 238 Define plugin manifest; 239 Add plugin discovery; 240 Add plugin lifecycle; 241 Add plugin permissions; 242 Add provider plugins; 243 Add tool plugins.

### Milestone 15 — Remote Runtime (PR 244–251)
244 Define runtime transport; 245 Define remote protocol; 246 Add authenticated daemon; 247 Add WebSocket event stream; 248 Add remote tool execution; 249 Add remote project support; 250 Add remote credential isolation; 251 Add node management UI.

### Milestone 16 — Production Hardening (não numerada na fonte)
Crash recovery, database backups, migrations, secret migration, rate limiting, resource limiting, audit logs, security tests, fuzz tests, load tests, workflow recovery tests, agent loop tests, provider compatibility tests, release signing e auto updater. A fila deve transformar estes itens em PRs sequenciais adicionais, sem assumir que os PRs 001–251 já os cobrem.

## Releases alvo

v0.1 Foundation (M0–M2); v0.2 Single-Agent Chat (M3–M4); v0.3 Agent Tools (M5–M6); v0.4 Memory + Skills (M7–M8); v0.5 Multi-Agent (M9); v0.6 Workflows + Scheduler (M10–M11); v0.7 Development Agents (M12); v0.8 Controlled Self-Evolution (M13); v0.9 Plugins + MCP + Remote (M14–M15); v1.0 Production (M16 + estabilização).

## Entregável esperado do planejamento

Specification Review formal com severidade BLOCKER/MAJOR/MINOR/SUGGESTION; SDD mestre sem perdas; fronteiras de camadas; queue executável com PRs pequenas e todos os campos exigidos; DAG sem dependências inexistentes; ondas paralelas e caminho crítico; demos verificáveis por release; Architecture Invariants; política de desenvolvimento automatizado; PR Execution Contract; PR #001 exata. Toda afirmação de implementação/executado deve ser marcada como não aplicável: neste momento somente planejamento será produzido.
