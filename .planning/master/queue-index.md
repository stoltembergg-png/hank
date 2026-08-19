# Índice da Executable PR Queue

**Status:** índice derivado dos três arquivos de queue; os cards são planejamento, não execução.

## Reconciliação de execução — 2026-08-19

- `main` final: `34525d2396747cb45d9c5001efbdf8e30880eb00`.
- PR-001..PR-004 foram formalmente merged após todos os required contexts verdes.
- A baseline executada também cobre os gates planejados para PR-005–PR-010 e PR-012: Rust fmt/Clippy/test/build, Frontend audit/lint/typecheck/test/build, CodeQL e Tauri.
- Esses cards permanecem rastreáveis como planejamento histórico e não devem gerar no-op.
- O próximo card não implementado identificado pela DAG é PR-013 (commits convencionais); PR-011 foi concluída em `b51b688` e PR-005–PR-010 e PR-012 não devem ser reexecutadas como trabalho duplicado.
- Evidência ONP final: run `32209782480`, artifact `onp-evidence-34525d2396747cb45d9c5001efbdf8e30880eb00`, `audit --ci` exit 0.
- Enforcement final exige os nove contexts reais documentados em `docs/development/global-blocker-closure.md`.


## Cobertura e validação

- 270 cards indexados exatamente uma vez: PR-001–PR-095 em `queue-001-095.md`, PR-096–PR-172 em `queue-096-172.md`, PR-173–PR-270 em `queue-173-270.md`.
- IDs são únicos, sequenciais e sem lacunas/duplicatas; categorias observadas estão no conjunto permitido.
- O índice aponta cada card para sua fatia original e preserva títulos/milestones; não altera a fila.
- Blockers documentais: PR-003–PR-019, PR-049–PR-055, PR-072, PR-074 e PR-091–PR-094 usam o rótulo `Arquivos prováveis`; PR-111, PR-121, PR-135, PR-154 e PR-172 usam condição para desbloquear a próxima milestone. O contrato exige `Arquivos/crates prováveis` e condição de próxima PR; normalização pendente é `NO_PROOF`.
- As seis referências para frente detectadas no texto de dependências são tratadas no [DAG](dependency-dag.md); não se presume execução.

## Mapa por PR

| PR | Milestone | Card |
|---|---|---|

| PR-001 | M0 — Foundation | [Inicializar workspace Cargo](../queue/queue-001-095.md#pr-001) |
| PR-002 | M0 — Foundation | [Inicializar desktop Tauri 2](../queue/queue-001-095.md#pr-002) |
| PR-003 | M0 — Foundation | [Adicionar workspace frontend](../queue/queue-001-095.md#pr-003) |
| PR-004 | M0 — Foundation | [Criar workflow de build](../queue/queue-001-095.md#pr-004) |
| PR-005 | M0 — Foundation | [Adicionar checks de Rust fmt](../queue/queue-001-095.md#pr-005) |
| PR-006 | M0 — Foundation | [Adicionar checks de Rust Clippy](../queue/queue-001-095.md#pr-006) |
| PR-007 | M0 — Foundation | [Adicionar workflow de testes Rust](../queue/queue-001-095.md#pr-007) |
| PR-008 | M0 — Foundation | [Adicionar lint frontend](../queue/queue-001-095.md#pr-008) |
| PR-009 | M0 — Foundation | [Adicionar typecheck frontend](../queue/queue-001-095.md#pr-009) |
| PR-010 | M0 — Foundation | [Adicionar testes frontend](../queue/queue-001-095.md#pr-010) |
| PR-011 | M0 — Foundation | [Adicionar Dependabot](../queue/queue-001-095.md#pr-011) |
| PR-012 | M0 — Foundation | [Adicionar CodeQL](../queue/queue-001-095.md#pr-012) |
| PR-013 | M0 — Foundation | [Adicionar conventional commits](../queue/queue-001-095.md#pr-013) |
| PR-014 | M0 — Foundation | [Adicionar validação de título de PR](../queue/queue-001-095.md#pr-014) |
| PR-015 | M0 — Foundation | [Adicionar automação de changelog](../queue/queue-001-095.md#pr-015) |
| PR-016 | M0 — Foundation | [Adicionar workflow de release](../queue/queue-001-095.md#pr-016) |
| PR-017 | M0 — Foundation | [Definir regras de contribuição](../queue/queue-001-095.md#pr-017) |
| PR-018 | M0 — Foundation | [Adicionar documentação de arquitetura](../queue/queue-001-095.md#pr-018) |
| PR-019 | M0 — Foundation | [Adicionar estrutura de ADR](../queue/queue-001-095.md#pr-019) |
| PR-020 | M0 — Foundation | [Adicionar framework de fixtures](../queue/queue-001-095.md#pr-020) |
| PR-021 | M1 — Core Domain | [Introduzir IDs tipados](../queue/queue-001-095.md#pr-021) |
| PR-022 | M1 — Core Domain | [Adicionar modelo de erro de domínio](../queue/queue-001-095.md#pr-022) |
| PR-023 | M1 — Core Domain | [Adicionar modelo de eventos de aplicação](../queue/queue-001-095.md#pr-023) |
| PR-024 | M1 — Core Domain | [Implementar event bus](../queue/queue-001-095.md#pr-024) |
| PR-025 | M1 — Core Domain | [Adicionar SQLite](../queue/queue-001-095.md#pr-025) |
| PR-026 | M1 — Core Domain | [Adicionar SQL migrations](../queue/queue-001-095.md#pr-026) |
| PR-027 | M1 — Core Domain | [Adicionar entidade Project](../queue/queue-001-095.md#pr-027) |
| PR-028 | M1 — Core Domain | [Adicionar Project repository](../queue/queue-001-095.md#pr-028) |
| PR-029 | M1 — Core Domain | [Adicionar create-project service](../queue/queue-001-095.md#pr-029) |
| PR-030 | M1 — Core Domain | [Adicionar list-project service](../queue/queue-001-095.md#pr-030) |
| PR-031 | M1 — Core Domain | [Adicionar update-project service](../queue/queue-001-095.md#pr-031) |
| PR-032 | M1 — Core Domain | [Adicionar archive-project service](../queue/queue-001-095.md#pr-032) |
| PR-033 | M1 — Core Domain | [Adicionar folders a Projects](../queue/queue-001-095.md#pr-033) |
| PR-034 | M1 — Core Domain | [Adicionar repositories a Projects](../queue/queue-001-095.md#pr-034) |
| PR-035 | M1 — Core Domain | [Adicionar project settings](../queue/queue-001-095.md#pr-035) |
| PR-036 | M1 — Core Domain | [Adicionar project UI listing](../queue/queue-001-095.md#pr-036) |
| PR-037 | M1 — Core Domain | [Adicionar create-project UI](../queue/queue-001-095.md#pr-037) |
| PR-038 | M1 — Core Domain | [Adicionar project detail UI](../queue/queue-001-095.md#pr-038) |
| PR-039 | M2 — Agent Domain | [Adicionar Agent entity](../queue/queue-001-095.md#pr-039) |
| PR-040 | M2 — Agent Domain | [Adicionar Agent repository](../queue/queue-001-095.md#pr-040) |
| PR-041 | M2 — Agent Domain | [Adicionar Agent configuration schema](../queue/queue-001-095.md#pr-041) |
| PR-042 | M2 — Agent Domain | [Adicionar personality schema](../queue/queue-001-095.md#pr-042) |
| PR-043 | M2 — Agent Domain | [Adicionar instruction hierarchy](../queue/queue-001-095.md#pr-043) |
| PR-044 | M2 — Agent Domain | [Adicionar tool permission schema](../queue/queue-001-095.md#pr-044) |
| PR-045 | M2 — Agent Domain | [Adicionar model policy schema](../queue/queue-001-095.md#pr-045) |
| PR-046 | M2 — Agent Domain | [Adicionar autonomy policy](../queue/queue-001-095.md#pr-046) |
| PR-047 | M2 — Agent Domain | [Adicionar budget policy](../queue/queue-001-095.md#pr-047) |
| PR-048 | M2 — Agent Domain | [Adicionar Agent CRUD services](../queue/queue-001-095.md#pr-048) |
| PR-049 | M2 — Agent Domain | [Adicionar agent list UI](../queue/queue-001-095.md#pr-049) |
| PR-050 | M2 — Agent Domain | [Adicionar Agent Builder identity page](../queue/queue-001-095.md#pr-050) |
| PR-051 | M2 — Agent Domain | [Adicionar personality page](../queue/queue-001-095.md#pr-051) |
| PR-052 | M2 — Agent Domain | [Adicionar model page](../queue/queue-001-095.md#pr-052) |
| PR-053 | M2 — Agent Domain | [Adicionar permissions page](../queue/queue-001-095.md#pr-053) |
| PR-054 | M2 — Agent Domain | [Adicionar instructions page](../queue/queue-001-095.md#pr-054) |
| PR-055 | M2 — Agent Domain | [Adicionar autonomy page](../queue/queue-001-095.md#pr-055) |
| PR-056 | M3 — Provider System | [Definir `ModelProvider` trait](../queue/queue-001-095.md#pr-056) |
| PR-057 | M3 — Provider System | [Definir model capability schema](../queue/queue-001-095.md#pr-057) |
| PR-058 | M3 — Provider System | [Definir normalized request](../queue/queue-001-095.md#pr-058) |
| PR-059 | M3 — Provider System | [Definir normalized response](../queue/queue-001-095.md#pr-059) |
| PR-060 | M3 — Provider System | [Definir streaming events](../queue/queue-001-095.md#pr-060) |
| PR-061 | M3 — Provider System | [Implementar OpenAI-compatible adapter](../queue/queue-001-095.md#pr-061) |
| PR-062 | M3 — Provider System | [Adicionar OpenAI provider](../queue/queue-001-095.md#pr-062) |
| PR-063 | M3 — Provider System | [Adicionar Anthropic provider](../queue/queue-001-095.md#pr-063) |
| PR-064 | M3 — Provider System | [Adicionar Gemini provider](../queue/queue-001-095.md#pr-064) |
| PR-065 | M3 — Provider System | [Adicionar OpenRouter provider](../queue/queue-001-095.md#pr-065) |
| PR-066 | M3 — Provider System | [Adicionar Ollama provider](../queue/queue-001-095.md#pr-066) |
| PR-067 | M3 — Provider System | [Implementar provider registry](../queue/queue-001-095.md#pr-067) |
| PR-068 | M3 — Provider System | [Adicionar credential service](../queue/queue-001-095.md#pr-068) |
| PR-069 | M3 — Provider System | [Adicionar encrypted secret storage](../queue/queue-001-095.md#pr-069) |
| PR-070 | M3 — Provider System | [Adicionar OAuth framework](../queue/queue-001-095.md#pr-070) |
| PR-071 | M3 — Provider System | [Adicionar OAuth callback handling](../queue/queue-001-095.md#pr-071) |
| PR-072 | M3 — Provider System | [Adicionar provider settings UI](../queue/queue-001-095.md#pr-072) |
| PR-073 | M3 — Provider System | [Adicionar model discovery](../queue/queue-001-095.md#pr-073) |
| PR-074 | M3 — Provider System | [Adicionar model selector](../queue/queue-001-095.md#pr-074) |
| PR-075 | M3 — Provider System | [Adicionar provider health check](../queue/queue-001-095.md#pr-075) |
| PR-076 | M3 — Provider System | [Implementar fallback policy](../queue/queue-001-095.md#pr-076) |
| PR-077 | M3 — Provider System | [Adicionar provider application/invocation service](../queue/queue-001-095.md#pr-077) |
| PR-078 | M4 — Chat Runtime | [Adicionar Session entity](../queue/queue-001-095.md#pr-078) |
| PR-079 | M4 — Chat Runtime | [Adicionar Message entity](../queue/queue-001-095.md#pr-079) |
| PR-080 | M4 — Chat Runtime | [Adicionar session storage](../queue/queue-001-095.md#pr-080) |
| PR-081 | M4 — Chat Runtime | [Adicionar message storage](../queue/queue-001-095.md#pr-081) |
| PR-082 | M4 — Chat Runtime | [Adicionar context builder interface](../queue/queue-001-095.md#pr-082) |
| PR-083 | M4 — Chat Runtime | [Adicionar basic context builder](../queue/queue-001-095.md#pr-083) |
| PR-084 | M4 — Chat Runtime | [Adicionar agent execution state machine](../queue/queue-001-095.md#pr-084) |
| PR-085 | M4 — Chat Runtime | [Adicionar provider streaming](../queue/queue-001-095.md#pr-085) |
| PR-086 | M4 — Chat Runtime | [Adicionar cancellation](../queue/queue-001-095.md#pr-086) |
| PR-087 | M4 — Chat Runtime | [Adicionar retry policy](../queue/queue-001-095.md#pr-087) |
| PR-088 | M4 — Chat Runtime | [Adicionar session service](../queue/queue-001-095.md#pr-088) |
| PR-089 | M4 — Chat Runtime | [Adicionar chat command](../queue/queue-001-095.md#pr-089) |
| PR-090 | M4 — Chat Runtime | [Adicionar streaming Tauri events](../queue/queue-001-095.md#pr-090) |
| PR-091 | M4 — Chat Runtime | [Adicionar chat UI](../queue/queue-001-095.md#pr-091) |
| PR-092 | M4 — Chat Runtime | [Adicionar markdown rendering](../queue/queue-001-095.md#pr-092) |
| PR-093 | M4 — Chat Runtime | [Adicionar code block rendering](../queue/queue-001-095.md#pr-093) |
| PR-094 | M4 — Chat Runtime | [Adicionar model/provider indicators](../queue/queue-001-095.md#pr-094) |
| PR-095 | M4 — Chat Runtime | [Adicionar token metrics](../queue/queue-001-095.md#pr-095) |
| PR-096 | M5 — Tools | [Define Tool trait](../queue/queue-096-172.md#pr-096) |
| PR-097 | M5 — Tools | [Define tool schema](../queue/queue-096-172.md#pr-097) |
| PR-098 | M5 — Tools | [Add tool registry](../queue/queue-096-172.md#pr-098) |
| PR-099 | M5 — Tools | [Add permission evaluator](../queue/queue-096-172.md#pr-099) |
| PR-100 | M5 — Tools | [Add filesystem read tool](../queue/queue-096-172.md#pr-100) |
| PR-101 | M5 — Tools | [Add filesystem write tool](../queue/queue-096-172.md#pr-101) |
| PR-102 | M5 — Tools | [Add directory listing](../queue/queue-096-172.md#pr-102) |
| PR-103 | M5 — Tools | [Add process execution primitive](../queue/queue-096-172.md#pr-103) |
| PR-104 | M5 — Tools | [Add terminal tool](../queue/queue-096-172.md#pr-104) |
| PR-105 | M5 — Tools | [Add HTTP tool](../queue/queue-096-172.md#pr-105) |
| PR-106 | M5 — Tools | [Add Git status tool](../queue/queue-096-172.md#pr-106) |
| PR-107 | M5 — Tools | [Add Git diff tool](../queue/queue-096-172.md#pr-107) |
| PR-108 | M5 — Tools | [Add Git commit tool](../queue/queue-096-172.md#pr-108) |
| PR-109 | M5 — Tools | [Add tool-call rendering](../queue/queue-096-172.md#pr-109) |
| PR-110 | M5 — Tools | [Add timeout handling](../queue/queue-096-172.md#pr-110) |
| PR-111 | M5 — Tools | [Add confirmation policies](../queue/queue-096-172.md#pr-111) |
| PR-112 | M6 — Python Runtime | [Define worker protocol](../queue/queue-096-172.md#pr-112) |
| PR-113 | M6 — Python Runtime | [Create Python worker](../queue/queue-096-172.md#pr-113) |
| PR-114 | M6 — Python Runtime | [Add JSON-RPC transport](../queue/queue-096-172.md#pr-114) |
| PR-115 | M6 — Python Runtime | [Add Python process lifecycle](../queue/queue-096-172.md#pr-115) |
| PR-116 | M6 — Python Runtime | [Add Python SDK](../queue/queue-096-172.md#pr-116) |
| PR-117 | M6 — Python Runtime | [Add Python tool registration](../queue/queue-096-172.md#pr-117) |
| PR-118 | M6 — Python Runtime | [Add Python execution tool](../queue/queue-096-172.md#pr-118) |
| PR-119 | M6 — Python Runtime | [Add dependency environment management](../queue/queue-096-172.md#pr-119) |
| PR-120 | M6 — Python Runtime | [Add Python logs](../queue/queue-096-172.md#pr-120) |
| PR-121 | M6 — Python Runtime | [Add Python permissions](../queue/queue-096-172.md#pr-121) |
| PR-122 | M7 — Memory | [Add memory entity](../queue/queue-096-172.md#pr-122) |
| PR-123 | M7 — Memory | [Add memory repository](../queue/queue-096-172.md#pr-123) |
| PR-124 | M7 — Memory | [Add memory type taxonomy](../queue/queue-096-172.md#pr-124) |
| PR-125 | M7 — Memory | [Add memory candidate extractor](../queue/queue-096-172.md#pr-125) |
| PR-126 | M7 — Memory | [Add memory importance scoring](../queue/queue-096-172.md#pr-126) |
| PR-127 | M7 — Memory | [Add deduplication](../queue/queue-096-172.md#pr-127) |
| PR-128 | M7 — Memory | [Add keyword retrieval](../queue/queue-096-172.md#pr-128) |
| PR-129 | M7 — Memory | [Add embedding interface](../queue/queue-096-172.md#pr-129) |
| PR-130 | M7 — Memory | [Add vector retrieval backend](../queue/queue-096-172.md#pr-130) |
| PR-131 | M7 — Memory | [Add context memory selector](../queue/queue-096-172.md#pr-131) |
| PR-132 | M7 — Memory | [Add memory UI](../queue/queue-096-172.md#pr-132) |
| PR-133 | M7 — Memory | [Add manual memory editing](../queue/queue-096-172.md#pr-133) |
| PR-134 | M7 — Memory | [Add project memory isolation](../queue/queue-096-172.md#pr-134) |
| PR-135 | M7 — Memory | [Add agent memory policies](../queue/queue-096-172.md#pr-135) |
| PR-136 | M8 — Skills | [Define skill manifest](../queue/queue-096-172.md#pr-136) |
| PR-137 | M8 — Skills | [Add skill parser](../queue/queue-096-172.md#pr-137) |
| PR-138 | M8 — Skills | [Add skill repository](../queue/queue-096-172.md#pr-138) |
| PR-139 | M8 — Skills | [Add skill loader](../queue/queue-096-172.md#pr-139) |
| PR-140 | M8 — Skills | [Add project skills](../queue/queue-096-172.md#pr-140) |
| PR-141 | M8 — Skills | [Add global skills](../queue/queue-096-172.md#pr-141) |
| PR-142 | M8 — Skills | [Add agent skill bindings](../queue/queue-096-172.md#pr-142) |
| PR-143 | M8 — Skills | [Add skill versioning](../queue/queue-096-172.md#pr-143) |
| PR-144 | M8 — Skills | [Add skill UI](../queue/queue-096-172.md#pr-144) |
| PR-145 | M8 — Skills | [Add skill editor](../queue/queue-096-172.md#pr-145) |
| PR-146 | M8 — Skills | [Add skill test framework](../queue/queue-096-172.md#pr-146) |
| PR-147 | M8 — Skills | [Add skill validation](../queue/queue-096-172.md#pr-147) |
| PR-148 | M8 — Skills | [Add skill creation tool](../queue/queue-096-172.md#pr-148) |
| PR-149 | M8 — Skills | [Add learning evaluator](../queue/queue-096-172.md#pr-149) |
| PR-150 | M8 — Skills | [Add skill candidate generation](../queue/queue-096-172.md#pr-150) |
| PR-151 | M8 — Skills | [Add autonomous skill test](../queue/queue-096-172.md#pr-151) |
| PR-152 | M8 — Skills | [Add skill activation policies](../queue/queue-096-172.md#pr-152) |
| PR-153 | M8 — Skills | [Add skill rollback](../queue/queue-096-172.md#pr-153) |
| PR-154 | M8 — Skills | [Add skill lifecycle curator](../queue/queue-096-172.md#pr-154) |
| PR-155 | M9 — Multi-Agent | [Add AgentGroup entity](../queue/queue-096-172.md#pr-155) |
| PR-156 | M9 — Multi-Agent | [Add group repository](../queue/queue-096-172.md#pr-156) |
| PR-157 | M9 — Multi-Agent | [Add group membership](../queue/queue-096-172.md#pr-157) |
| PR-158 | M9 — Multi-Agent | [Add group session](../queue/queue-096-172.md#pr-158) |
| PR-159 | M9 — Multi-Agent | [Add mention parser](../queue/queue-096-172.md#pr-159) |
| PR-160 | M9 — Multi-Agent | [Add agent invocation protocol](../queue/queue-096-172.md#pr-160) |
| PR-161 | M9 — Multi-Agent | [Add delegation tool](../queue/queue-096-172.md#pr-161) |
| PR-162 | M9 — Multi-Agent | [Add invocation graph](../queue/queue-096-172.md#pr-162) |
| PR-163 | M9 — Multi-Agent | [Add cycle detection](../queue/queue-096-172.md#pr-163) |
| PR-164 | M9 — Multi-Agent | [Add maximum delegation depth](../queue/queue-096-172.md#pr-164) |
| PR-165 | M9 — Multi-Agent | [Add parallel invocation](../queue/queue-096-172.md#pr-165) |
| PR-166 | M9 — Multi-Agent | [Add group budgets](../queue/queue-096-172.md#pr-166) |
| PR-167 | M9 — Multi-Agent | [Add moderator policy](../queue/queue-096-172.md#pr-167) |
| PR-168 | M9 — Multi-Agent | [Add round policy](../queue/queue-096-172.md#pr-168) |
| PR-169 | M9 — Multi-Agent | [Add synthesis mode](../queue/queue-096-172.md#pr-169) |
| PR-170 | M9 — Multi-Agent | [Add group chat UI](../queue/queue-096-172.md#pr-170) |
| PR-171 | M9 — Multi-Agent | [Render agent-to-agent messages](../queue/queue-096-172.md#pr-171) |
| PR-172 | M9 — Multi-Agent | [Render delegation graph](../queue/queue-096-172.md#pr-172) |
| PR-173 | M10 — Workflow Engine | [Define workflow entity](../queue/queue-173-270.md#pr-173) |
| PR-174 | M10 — Workflow Engine | [Define workflow node](../queue/queue-173-270.md#pr-174) |
| PR-175 | M10 — Workflow Engine | [Define workflow edge](../queue/queue-173-270.md#pr-175) |
| PR-176 | M10 — Workflow Engine | [Add workflow persistence](../queue/queue-173-270.md#pr-176) |
| PR-177 | M10 — Workflow Engine | [Add execution engine](../queue/queue-173-270.md#pr-177) |
| PR-178 | M10 — Workflow Engine | [Add AgentNode](../queue/queue-173-270.md#pr-178) |
| PR-179 | M10 — Workflow Engine | [Add ToolNode](../queue/queue-173-270.md#pr-179) |
| PR-180 | M10 — Workflow Engine | [Add PythonNode](../queue/queue-173-270.md#pr-180) |
| PR-181 | M10 — Workflow Engine | [Add ConditionNode](../queue/queue-173-270.md#pr-181) |
| PR-182 | M10 — Workflow Engine | [Add ParallelNode](../queue/queue-173-270.md#pr-182) |
| PR-183 | M10 — Workflow Engine | [Add DelayNode](../queue/queue-173-270.md#pr-183) |
| PR-184 | M10 — Workflow Engine | [Add ApprovalNode](../queue/queue-173-270.md#pr-184) |
| PR-185 | M10 — Workflow Engine | [Add SubWorkflowNode](../queue/queue-173-270.md#pr-185) |
| PR-186 | M10 — Workflow Engine | [Add workflow state persistence](../queue/queue-173-270.md#pr-186) |
| PR-187 | M10 — Workflow Engine | [Add crash recovery](../queue/queue-173-270.md#pr-187) |
| PR-188 | M10 — Workflow Engine | [Add workflow logs](../queue/queue-173-270.md#pr-188) |
| PR-189 | M10 — Workflow Engine | [Add workflow editor](../queue/queue-173-270.md#pr-189) |
| PR-190 | M10 — Workflow Engine | [Add workflow run viewer](../queue/queue-173-270.md#pr-190) |
| PR-191 | M11 — Scheduler | [Add scheduled job entity](../queue/queue-173-270.md#pr-191) |
| PR-192 | M11 — Scheduler | [Add interval scheduling](../queue/queue-173-270.md#pr-192) |
| PR-193 | M11 — Scheduler | [Add cron parsing](../queue/queue-173-270.md#pr-193) |
| PR-194 | M11 — Scheduler | [Add one-shot scheduling](../queue/queue-173-270.md#pr-194) |
| PR-195 | M11 — Scheduler | [Add scheduler persistence](../queue/queue-173-270.md#pr-195) |
| PR-196 | M11 — Scheduler | [Add scheduler worker](../queue/queue-173-270.md#pr-196) |
| PR-197 | M11 — Scheduler | [Add missed-run policy](../queue/queue-173-270.md#pr-197) |
| PR-198 | M11 — Scheduler | [Add concurrent-run protection](../queue/queue-173-270.md#pr-198) |
| PR-199 | M11 — Scheduler | [Add workflow scheduler integration](../queue/queue-173-270.md#pr-199) |
| PR-200 | M11 — Scheduler | [Add agent scheduler integration](../queue/queue-173-270.md#pr-200) |
| PR-201 | M11 — Scheduler | [Add automation UI](../queue/queue-173-270.md#pr-201) |
| PR-202 | M11 — Scheduler | [Add execution history](../queue/queue-173-270.md#pr-202) |
| PR-203 | M11 — Scheduler | [Add desktop notifications](../queue/queue-173-270.md#pr-203) |
| PR-204 | M12 — Development Agents | [Add repository workspace manager](../queue/queue-173-270.md#pr-204) |
| PR-205 | M12 — Development Agents | [Add Git worktree manager](../queue/queue-173-270.md#pr-205) |
| PR-206 | M12 — Development Agents | [Add branch policy](../queue/queue-173-270.md#pr-206) |
| PR-207 | M12 — Development Agents | [Add task-to-branch mapping](../queue/queue-173-270.md#pr-207) |
| PR-208 | M12 — Development Agents | [Add coding agent profile](../queue/queue-173-270.md#pr-208) |
| PR-209 | M12 — Development Agents | [Add reviewer agent profile](../queue/queue-173-270.md#pr-209) |
| PR-210 | M12 — Development Agents | [Add QA agent profile](../queue/queue-173-270.md#pr-210) |
| PR-211 | M12 — Development Agents | [Add security agent profile](../queue/queue-173-270.md#pr-211) |
| PR-212 | M12 — Development Agents | [Add architecture agent profile](../queue/queue-173-270.md#pr-212) |
| PR-213 | M12 — Development Agents | [Add PR generation workflow](../queue/queue-173-270.md#pr-213) |
| PR-214 | M12 — Development Agents | [Add review workflow](../queue/queue-173-270.md#pr-214) |
| PR-215 | M12 — Development Agents | [Add CI status integration](../queue/queue-173-270.md#pr-215) |
| PR-216 | M12 — Development Agents | [Add fix-review workflow](../queue/queue-173-270.md#pr-216) |
| PR-217 | M12 — Development Agents | [Add release-agent workflow](../queue/queue-173-270.md#pr-217) |
| PR-218 | M13 — Controlled Autonomous Evolution | [Add improvement observation event](../queue/queue-173-270.md#pr-218) |
| PR-219 | M13 — Controlled Autonomous Evolution | [Add improvement candidate entity](../queue/queue-173-270.md#pr-219) |
| PR-220 | M13 — Controlled Autonomous Evolution | [Add self-evaluation workflow](../queue/queue-173-270.md#pr-220) |
| PR-221 | M13 — Controlled Autonomous Evolution | [Add skill improvement proposal](../queue/queue-173-270.md#pr-221) |
| PR-222 | M13 — Controlled Autonomous Evolution | [Add workflow improvement proposal](../queue/queue-173-270.md#pr-222) |
| PR-223 | M13 — Controlled Autonomous Evolution | [Add agent configuration proposal](../queue/queue-173-270.md#pr-223) |
| PR-224 | M13 — Controlled Autonomous Evolution | [Add automated evaluation](../queue/queue-173-270.md#pr-224) |
| PR-225 | M13 — Controlled Autonomous Evolution | [Add regression evaluation](../queue/queue-173-270.md#pr-225) |
| PR-226 | M13 — Controlled Autonomous Evolution | [Add improvement scoring](../queue/queue-173-270.md#pr-226) |
| PR-227 | M13 — Controlled Autonomous Evolution | [Add automatic skill rollout](../queue/queue-173-270.md#pr-227) |
| PR-228 | M13 — Controlled Autonomous Evolution | [Add automatic rollback](../queue/queue-173-270.md#pr-228) |
| PR-229 | M13 — Controlled Autonomous Evolution | [Add self-development issue creation](../queue/queue-173-270.md#pr-229) |
| PR-230 | M13 — Controlled Autonomous Evolution | [Add self-development branch creation](../queue/queue-173-270.md#pr-230) |
| PR-231 | M13 — Controlled Autonomous Evolution | [Add self-development PR creation](../queue/queue-173-270.md#pr-231) |
| PR-232 | M14 — MCP and Plugins | [Add MCP transport abstraction](../queue/queue-173-270.md#pr-232) |
| PR-233 | M14 — MCP and Plugins | [Add MCP stdio client](../queue/queue-173-270.md#pr-233) |
| PR-234 | M14 — MCP and Plugins | [Add MCP HTTP client](../queue/queue-173-270.md#pr-234) |
| PR-235 | M14 — MCP and Plugins | [Add MCP tool discovery](../queue/queue-173-270.md#pr-235) |
| PR-236 | M14 — MCP and Plugins | [Add MCP permission integration](../queue/queue-173-270.md#pr-236) |
| PR-237 | M14 — MCP and Plugins | [Add MCP settings UI](../queue/queue-173-270.md#pr-237) |
| PR-238 | M14 — MCP and Plugins | [Define plugin manifest](../queue/queue-173-270.md#pr-238) |
| PR-239 | M14 — MCP and Plugins | [Add plugin discovery](../queue/queue-173-270.md#pr-239) |
| PR-240 | M14 — MCP and Plugins | [Add plugin lifecycle](../queue/queue-173-270.md#pr-240) |
| PR-241 | M14 — MCP and Plugins | [Add plugin permissions](../queue/queue-173-270.md#pr-241) |
| PR-242 | M14 — MCP and Plugins | [Add provider plugins](../queue/queue-173-270.md#pr-242) |
| PR-243 | M14 — MCP and Plugins | [Add tool plugins](../queue/queue-173-270.md#pr-243) |
| PR-244 | M15 — Remote Runtime | [Define runtime transport](../queue/queue-173-270.md#pr-244) |
| PR-245 | M15 — Remote Runtime | [Define remote protocol](../queue/queue-173-270.md#pr-245) |
| PR-246 | M15 — Remote Runtime | [Add authenticated daemon](../queue/queue-173-270.md#pr-246) |
| PR-247 | M15 — Remote Runtime | [Add WebSocket event stream](../queue/queue-173-270.md#pr-247) |
| PR-248 | M15 — Remote Runtime | [Add remote tool execution](../queue/queue-173-270.md#pr-248) |
| PR-249 | M15 — Remote Runtime | [Add remote project support](../queue/queue-173-270.md#pr-249) |
| PR-250 | M15 — Remote Runtime | [Add remote credential isolation](../queue/queue-173-270.md#pr-250) |
| PR-251 | M15 — Remote Runtime | [Add node management UI](../queue/queue-173-270.md#pr-251) |
| PR-252 | M16 — Production Hardening | [Harden application crash recovery](../queue/queue-173-270.md#pr-252) |
| PR-253 | M16 — Production Hardening | [Add database backups](../queue/queue-173-270.md#pr-253) |
| PR-254 | M16 — Production Hardening | [Add backup restore](../queue/queue-173-270.md#pr-254) |
| PR-255 | M16 — Production Hardening | [Add migration hardening](../queue/queue-173-270.md#pr-255) |
| PR-256 | M16 — Production Hardening | [Add secret migration](../queue/queue-173-270.md#pr-256) |
| PR-257 | M16 — Production Hardening | [Add rate limiting](../queue/queue-173-270.md#pr-257) |
| PR-258 | M16 — Production Hardening | [Add resource limiting](../queue/queue-173-270.md#pr-258) |
| PR-259 | M16 — Production Hardening | [Add audit logs](../queue/queue-173-270.md#pr-259) |
| PR-260 | M16 — Production Hardening | [Add security tests](../queue/queue-173-270.md#pr-260) |
| PR-261 | M16 — Production Hardening | [Add fuzz tests](../queue/queue-173-270.md#pr-261) |
| PR-262 | M16 — Production Hardening | [Add load tests](../queue/queue-173-270.md#pr-262) |
| PR-263 | M16 — Production Hardening | [Add workflow recovery tests](../queue/queue-173-270.md#pr-263) |
| PR-264 | M16 — Production Hardening | [Add agent loop tests](../queue/queue-173-270.md#pr-264) |
| PR-265 | M16 — Production Hardening | [Add provider compatibility tests](../queue/queue-173-270.md#pr-265) |
| PR-266 | M16 — Production Hardening | [Add release signing](../queue/queue-173-270.md#pr-266) |
| PR-267 | M16 — Production Hardening | [Add installers](../queue/queue-173-270.md#pr-267) |
| PR-268 | M16 — Production Hardening | [Add auto updater](../queue/queue-173-270.md#pr-268) |
| PR-269 | M16 — Production Hardening | [Add release rollback](../queue/queue-173-270.md#pr-269) |
| PR-270 | M16 — Production Hardening | [Add distribution gates](../queue/queue-173-270.md#pr-270) |

## Gates de uso

Um link de card só é elegível quando seus predecessores normalizados, contratos bloqueadores, checks e review independente estão evidenciados no mesmo SHA/tree/policy. A ausência de prova, dependência sem contrato, campo obrigatório divergente, falha de CI/security/architecture ou resultado stale mantém o card bloqueado.
