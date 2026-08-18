# Spec: Tauri Desktop

> feature: tauri-desktop
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

Esta feature implementa PR-002 da queue executável: inicializar o shell desktop Tauri 2 mínimo como consumidor futuro da Application API. O Tauri NÃO é o core (AI-001, D-001) — é apenas shell/adaptador de bridge, eventos, janela, deep links e packaging. Esta feature estabelece a fronteira explícita entre UI privilegiada e core.

## Histórias

### US-101 — Shell desktop Tauri 2 funcional e isolado

Como desenvolvedor do core, quero um shell Tauri 2 que abra/fecha janela de forma determinística e rejeite origens remotas não previstas, para que eu tenha uma fronteira explícita entre UI privilegiada e core sem acoplamento a SQLite, providers ou rede.

#### AC-101 — Janela abre e fecha determinísticamente

- **Dado** um app Tauri 2 recém-inicializado
- **Quando** executo o app em ambiente suportado
- **Então** a janela abre, permanece responsiva e fecha sem erros; exit code 0

#### AC-102 — Manifest Tauri não concede capacidades perigosas

- **Dado** o `tauri.conf.json` gerado
- **Quando** inspeciono capabilities, permissions e CSP
- **Então** não há `allowlist` com `all: true`, `fs: all`, `process: all`, `network: all`, `shell: all` ou `dialog: all`; apenas capabilities mínimas de janela

#### AC-103 — Origem remota não prevista é rejeitada

- **Dado** o app Tauri rodando
- **Quando** tento carregar uma URL externa não declarada no `tauri.conf.json`
- **Então** o carregamento falha (CSP/origin policy bloqueia)

#### AC-104 — Bridge Tauri→Application API preparada (sem comandos de produto)

- **Dado** o `tauri.conf.json` e código Rust do Tauri
- **Quando** inspeciono `invoke` handlers registrados
- **Então** não há handlers de produto (chat, agent, tools); apenas infraestrutura de lifecycle/ready

#### AC-105 — Logs estruturados de boot/ready/close sem payloads sensíveis

- **Dado** o app Tauri rodando
- **Quando** observo stdout/stderr durante boot, ready e close
- **Então** logs contêm timestamp, nível, evento (boot/ready/close/falha) e versão; sem tokens, URLs, segredos ou conteúdo de usuário

## Fora de escopo

- Chat, Agent Runtime, sessões, mensagens
- Acesso direto a SQLite, filesystem, providers, navegação remota
- Secrets, OAuth, credentials
- Qualquer feature de produto além do shell minimal

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-101 | Rust 1.97+ e Node.js 20+ disponíveis no ambiente | aberta | — |
| ASM-102 | Tauri 2.x estável suporta as capabilities mínimas necessárias | aberta | — |
| ASM-103 | O core Rust (agent-core) já compila e está disponível via workspace | aberta | PR-001 preparado e validado localmente; merge e CI ainda pendentes |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-101 | Qual versão exata do Tauri 2 (2.0.x, 2.1.x)? | aberta | — |
| Q-102 | O frontend será TypeScript/React, Svelte, ou vanilla? | aberta | — |
| Q-103 | Haverá suporte a macOS/Windows/Linux desde v0.1? | aberta | — |