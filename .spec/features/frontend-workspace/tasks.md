# Tasks: Frontend Workspace

> feature: frontend-workspace

<!--
  Como ler este arquivo (o formato é verificado por `onp-spec audit`):
  - T-xxx = tarefa (código de rastreio, único no projeto inteiro).
  - Toda tarefa referencia em `Refs:` pelo menos uma história de usuário
    (US-xxx) ou critério de aceite (AC-xxx).
  - Toda tarefa lista os arquivos que cria/altera em `Arquivos:` — capriche:
    é o que decide o que `onp-spec plano` roda em PARALELO (arquivos
    disjuntos) e o que roda em sequência.
  - Campos opcionais por tarefa, usados pelo plano de execução:
    `- Modelo: claude-sonnet-5` e `- Esforço: alto` (baixo|medio|alto|xalto|max).
  - Uma tarefa só pode virar [concluida] quando os critérios de aceite dela
    tiverem prova PASS registrada por `onp-spec verify`.
  Status: pendente | em-andamento | concluida
    (atalho: `onp-spec tarefa <feature> <T-xxx> <status>`)
-->

## T-321 — Inicializar workspace frontend (package.json, tsconfig, build tool) [pendente]

- Refs: US-201, AC-201, AC-202
- Arquivos: frontend/package.json, frontend/package-lock.json, frontend/tsconfig.json, frontend/tsconfig.node.json, frontend/vite.config.ts, frontend/vitest.config.ts, frontend/index.html, frontend/src/main.tsx, frontend/src/App.tsx, frontend/src/App.css
- Notas: TypeScript strict mode; build tool (Vite); type-check script

## T-322 — Configurar lint e type-check scripts [pendente]

- Refs: US-201, AC-201, AC-202
- Arquivos: frontend/.eslintrc.json, frontend/tsconfig.json, frontend/package.json, frontend/package-lock.json, frontend/vitest.config.ts
- Notas: ESLint + TypeScript strict; `npm run lint` e `npm run typecheck` passam

## T-323 — Fixture de imports proibidos (sqlite/fs/tauri/providers) [pendente]

- Refs: US-201, AC-203
- Arquivos: frontend/tests/frontend_ac_tests.test.ts
- Notas: Teste executa busca recursiva real sobre frontend/src

## T-324 — CSP e capabilities mínimas no manifest Tauri [pendente]

- Refs: US-201, AC-204
- Arquivos: frontend/tests/frontend_ac_tests.test.ts
- Notas: Teste verifica o manifest v2 fornecido por PR-002; não altera o predecessor

## T-325 — Testes de aceitação: build, lint, typecheck, CSP, logs [pendente]

- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/tests/frontend_ac_tests.test.ts, frontend/src/App.tsx, frontend/src/App.css
- Notas: Testes cobrindo AC-201..205: build/lint/typecheck, CSP, logs

## T-326 — Adicionar UI de listagem de Projects [pendente]

- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/ProjectList.tsx, frontend/tests/project_list_contract.test.ts, docs/project-list-ui.md
- Notas: Lista bounded via application service injetado; estados loading/empty/error/ready; sem acesso a storage/Tauri/provider

## T-327 — Adicionar UI de criação de Projects [pendente]

- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/CreateProjectForm.tsx, frontend/tests/create_project_contract.test.ts, docs/create-project-ui.md
- Notas: DTO allowlisted, validação bounded, submit lock e estados validation/conflict/error/success via service injetado

## T-328 — Adicionar UI de detalhe de Projects [pendente]

- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/ProjectDetail.tsx, frontend/tests/project_detail_contract.test.ts, docs/project-detail-ui.md
- Notas: Update/archive via services injetados, version check, confirmação explícita e estado archived terminal

## T-338 — Mapear implementação legacy de Project list [pendente]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/projects.ts, frontend/src/components/ProjectList.tsx, frontend/src/types/project.ts, frontend/tests/project_list_ac_tests.test.ts
- Notas: Project-scoped list component/service contract; loading/empty/error/pagination; sem acesso direto a storage

## T-339 — Mapear implementação legacy de Project create [pendente]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/projects.ts, frontend/src/components/CreateProjectForm.tsx, frontend/src/components/ProjectList.tsx, frontend/src/types/project.ts, frontend/tests/create_project_ac_tests.test.ts, frontend/tests/project_list_ac_tests.test.ts
- Notas: Create DTO allowlisted, validation, submit state e service boundary; sem acesso direto a storage

## T-340 — Mapear implementação legacy de Project detail [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/projects.ts, frontend/src/components/CreateProjectForm.tsx, frontend/src/components/ProjectDetailView.tsx, frontend/src/components/ProjectList.tsx, frontend/src/types/project.ts, frontend/tests/create_project_ac_tests.test.ts, frontend/tests/project_detail_ac_tests.test.ts, frontend/tests/project_list_ac_tests.test.ts
- Notas: Detail update/archive com optimistic version, confirmação explícita e estados conflict/error; sem acesso direto a storage

## T-342 — Adicionar UI de listagem de Agents [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/agents.ts, frontend/src/components/AgentList.tsx, frontend/src/types/agent.ts, frontend/tests/agent_list_ac_tests.test.tsx
- Notas: Project-scoped list via AgentApiClient; loading/empty/error/pagination; sem acesso direto a storage/Tauri/provider

## T-343 — Adicionar página de identidade do Agent Builder [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/agents/builder/identity/AgentIdentityPage.tsx, frontend/src/agents/builder/identity/types.ts, frontend/src/agents/builder/identity/AgentIdentityPage.css, frontend/tests/agent_identity_ac_tests.test.tsx, docs/agent-identity-page.md
- Notas: Form with name/description, validation, optimistic version, cancel/confirm, stale/archived/permission handling; update service as sole write path

## T-344 — Adicionar página de personalidade do Agent [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/agents/builder/personality/PersonalityPage.tsx, frontend/src/agents/builder/personality/types.ts, frontend/src/agents/builder/personality/PersonalityPage.css, frontend/tests/agent_personality_ac_tests.test.tsx, docs/agent-personality-page.md
- Notas: Personality bounded, plain-text preview, Agent-layer precedence warning, injection/secret rejection, stale-safe update service and inactive protection

## T-345 — Adicionar página de política de modelo do Agent [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/agent-model-policy.ts, frontend/src/agents/builder/model/ModelPolicyPage.tsx, frontend/src/agents/builder/model/types.ts, frontend/src/agents/builder/model/ModelPolicyPage.css, frontend/tests/agent_model_policy_ac_tests.test.tsx, docs/agent-model-policy-page.md
- Notas: Provider-neutral IDs, bounded limits/modalities, explicit capability/provider state, no credentials/endpoints, stale-safe typed service boundary

## T-346 — Adicionar página de permissões do Agent [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/agent-tool-permissions.ts, frontend/src/agents/builder/permissions/PermissionsPage.tsx, frontend/src/agents/builder/permissions/types.ts, frontend/src/agents/builder/permissions/PermissionsPage.css, frontend/tests/agent_permissions_ac_tests.test.tsx, docs/agent-permissions-page.md
- Notas: Default deny imutável, allow/ask/deny, escopo bounded, conflitos/wildcards rejeitados, grants sensíveis exigem ask/deny e estado unsupported explícito

## T-347 — Adicionar editor de instruções da camada Agent [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/agent-instructions.ts, frontend/src/agents/builder/instructions/InstructionsPage.tsx, frontend/src/agents/builder/instructions/types.ts, frontend/src/agents/builder/instructions/InstructionsPage.css, frontend/tests/agent_instructions_ac_tests.test.tsx, docs/agent-instructions-page.md
- Notas: Camada Agent fixa, budget bounded, provenance, preview plain-text não confiável, stale protection e ausência de system/security/prompt send

## T-348 — Adicionar página de autonomia do Agent [concluida]
- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/api/agent-autonomy.ts, frontend/src/agents/builder/autonomy/AutonomyPage.tsx, frontend/src/agents/builder/autonomy/types.ts, frontend/src/agents/builder/autonomy/AutonomyPage.css, frontend/tests/agent_autonomy_ac_tests.test.tsx, docs/agent-autonomy-page.md
- Notas: L0-L4, matriz de decisões, downgrade reversível, escalação com approval bounded, stale/unsupported fail-closed e sem autoelevação

## T-1301 — Integrar Agents ao detalhe do Project [em-andamento]

- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/components/ProjectDetailView.tsx, frontend/src/components/ProjectDetailView.css, frontend/tests/project_agents_workbench.test.tsx
- Notas: Aba acessível e project-scoped para abrir a listagem de Agents existente; client injetável preserva a fronteira de serviços e não cria dados sintéticos.

## T-1385 — Criar Agent pelo workbench do Project [em-andamento]

- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/components/AgentList.tsx, frontend/src/components/AgentList.css, frontend/tests/agent_creation_workbench.test.tsx
- Notas: formulário bounded e acessível usa o AgentApiClient injetado, envia somente project_id/nome/descrição e recarrega a listagem após confirmação do serviço.

## T-1387 — Tornar o lint do frontend portátil [em-andamento]

- Refs: US-201, AC-201, AC-202
- Arquivos: frontend/package.json, frontend/tests/frontend_ac_tests.test.ts
- Notas: o script `lint` não depende de atribuição de variável POSIX, permitindo executar o mesmo gate em Windows e CI.

## T-1389 — Integrar fundação do Product Shell [em-andamento]

- Refs: US-201, AC-201, AC-202, AC-203, AC-204, AC-205
- Arquivos: frontend/src/App.tsx, frontend/src/App.css, frontend/src/components/ProductShell.tsx, frontend/src/components/ProductShell.css, frontend/tests/product_shell.test.tsx
- Notas: shell persistente com navegação acessível, status de sessão local e conteúdo Projects real; módulos ainda não integrados ficam desabilitados e explicitamente identificados, sem rotas ou dados fictícios.
