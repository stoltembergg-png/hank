# Spec: Frontend Workspace

> feature: frontend-workspace
> status: rascunho

<!--
  Como ler este arquivo (o formato é verificado por `onp-spec audit`):
  - US-xxx = história de usuário · AC-xxx = critério de aceite
    ASM-xxx = suposição · Q-xxx = pergunta em aberto
    São códigos de rastreio: ligam a especificação às tarefas e aos testes.
  - Toda história de usuário precisa de pelo menos um critério de aceite.
  - Todo critério de aceite precisa de Dado/Quando/Então completos.
  - Os códigos são únicos no projeto inteiro (nunca reutilize um número).
  - Suposições e Perguntas em aberto são OBRIGATÓRIAS: se não há nenhuma,
    escreva "Nenhuma." — mas desconfie: quase toda feature esconde uma.
-->

## Contexto

Esta feature implementa PR-003 da queue executável: criar o workspace frontend tipado que só consumirá contratos da Application API. O frontend NÃO acessa SQLite, filesystem, providers ou Tauri diretamente (AI-002, AI-035) — só consome contratos via bridge tipada.

## Histórias

### US-201 — Workspace frontend compilável e desacoplado

Como desenvolvedor do frontend, quero um workspace que compile, faça lint e type-check sem acoplar a Tauri, SQLite, providers ou filesystem, para que a UI permaneça desacoplada do core.

#### AC-201 — Build e lint inicial passam

- **Dado** um workspace frontend recém-inicializado
- **Quando** executo `npm run build` e `npm run lint`
- **Então** ambos terminam com exit code 0; sem erros de tipo ou lint

#### AC-202 — Type safety: sem `any` explícito em código de produto

- **Dado** o código fonte do frontend
- **Quando** executo type-check estrito (`tsc --noEmit` ou equivalente)
- **Então** passa sem erros; `any` só permitido em boundaries de bridge tipada

#### AC-203 — Busca estática não encontra imports proibidos

- **Dado** o código fonte do frontend
- **Quando** busco por imports de `sqlite`, `fs`, `path`, `tauri`, `sqlx`, providers concretos
- **Então** nenhum match em código de produto (apenas em fixtures de teste se houver)

#### AC-204 — CSP e capabilities mínimas declaradas

- **Dado** o `tauri.conf.json` do app desktop (PR-002)
- **Quando** inspeciono CSP e capabilities do frontend
- **Então** CSP restritivo (`default-src 'self'`, sem `unsafe-inline/eval`), capabilities só de UI

#### AC-205 — Eventos de montagem/falha com versão e sem conteúdo de usuário

- **Dado** o frontend montando
- **Quando** observo console/logs durante mount e unmount
- **Então** eventos contêm timestamp, evento (mount/unmount/error), versão do bundle; sem tokens, URLs, PII

## Fora de escopo

- Telas completas (chat, agent builder, settings)
- Chamadas SQLite, `invoke` genérico, lógica de Agent Runtime
- Qualquer feature de produto além do workspace shell

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-201 | Node.js 20+ e package manager (npm/pnpm/yarn) disponíveis | aberta | — |
| ASM-202 | Framework frontend será TypeScript-based (React/Svelte/Vue/vanilla) | confirmada | Implementação usa React + TypeScript |
| ASM-203 | PR-002 (Tauri desktop) já existe e expõe bridge tipada | aberta | Branch PR-002 empilhada sobre Foundation; merge e CI ainda pendentes |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-201 | Qual framework: React, Svelte, Vue, vanilla TS? | resolvida | React + TypeScript |
| Q-202 | Build tool: Vite, Webpack, Turbopack? | resolvida | Vite |
| Q-203 | Testing: Vitest, Playwright, Cypress? | resolvida | Vitest |