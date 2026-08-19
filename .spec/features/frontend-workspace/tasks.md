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