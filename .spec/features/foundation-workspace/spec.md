# Spec: Foundation workspace

> feature: foundation-workspace
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

Esta feature implementa PR-001 da queue executável: inicializar o workspace Cargo modular mínimo para o core Rust reutilizável da plataforma desktop multiagente. O workspace define a fronteira arquitetural entre domínio (agent-core), runtime (agent-runtime), protocolo (agent-protocol), suporte a testes (test-support) e automação (xtask), garantindo que o Tauri não seja dependência do core (AI-001, AI-003, D-001, D-002).

## Histórias

### US-301 — Workspace Cargo compilável e verificável

Como desenvolvedor do core, quero um workspace Cargo com crates isoladas e resolver determinístico, para que eu possa compilar, testar e auditar o core sem dependências de Tauri, providers concretos ou efeitos de rede/filesystem privilegiados.

#### AC-301 — Metadata do workspace lista apenas crates planejadas @spec:AC-301

- **Dado** um workspace Cargo recém-inicializado
- **Quando** executo `cargo metadata --format-version=1`
- **Então** a saída enumera exatamente os packages: `agent-core`, `agent-runtime`, `agent-protocol`, `provider-core`, `provider-adapter-openai-compatible`, `provider-adapter-openai`, `provider-adapter-anthropic`, `provider-adapter-gemini`, `provider-adapter-openrouter`, `provider-adapter-ollama`, `tool-core`, `test-support` (dev-only) e `xtask`, sem crates extras ou placeholders silenciosos

#### AC-302 — Workspace compila sem erros em modo check @spec:AC-302

- **Dado** o workspace com crates mínimas
- **Quando** executo `cargo check --workspace`
- **Então** o comando termina com exit code 0 e sem warnings de dependências não usadas ou ciclos

#### AC-303 — Grafo de dependências respeita fronteiras arquiteturais @spec:AC-303

- **Dado** o workspace compilado
- **Quando** executo a fixture de arquitetura (forbidden-import test)
- **Então** o teste falha se `agent-core` importar `tao`, `tauri`, `wry`, `sqlx`, `tokio` (runtime), provedores concretos ou segredos de ambiente; e passa confirmando que `agent-core` depende apenas de `agent-protocol` e tipos estáveis

#### AC-304 — Teste de ciclo proibido detecta dependências circulares @spec:AC-304

- **Dado** o workspace com dependências declaradas
- **Quando** executo o teste de ciclo do grafo
- **Então** o teste passa (sem ciclos) e falharia se uma dependência circular fosse introduzida entre as crates do workspace

#### AC-305 — Toolchain e resolver determinísticos declarados @spec:AC-305

- **Dado** o `Cargo.toml` raiz e `Cargo.lock`
- **Quando** inspeciono o workspace
- **Então** o resolver está configurado como `resolver = "2"`, a toolchain mínima está declarada (via `rust-toolchain.toml` ou `rustup`), e `Cargo.lock` está versionado

## Fora de escopo

- Comportamento de agente, sessões, mensagens ou execução de ferramentas
- Tauri funcional, janela desktop, bridge de comandos ou frontend
- Providers (OpenAI, Anthropic, etc.), credenciais, OAuth ou streaming
- Persistência (SQLite, migrations, repositories), event bus ou workflows
- Código Python, sandbox, MCP, plugins ou runtime remoto
- Qualquer feature de produto além da estrutura do workspace

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-301 | Rust 1.80+ está disponível no ambiente de build/CI | aberta | — |
| ASM-302 | O preset base da constituição cobre princípios mínimos; princípios de arquitetura (AI-001, AI-003, AI-035) serão adicionados como verificação(proibido) depois | aberta | — |
| ASM-303 | `test-support` como dev-only não vaza para dependências de produção | aberta | — |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-301 | Qual versão mínima do Rust (MSRV) o projeto vai fixar? | aberta | — |
| Q-302 | `xtask` será crate binária ou library + bin? | aberta | — |
| Q-303 | O workspace usará `cargo-workspaces` features (ex: `workspace.dependencies`, `workspace.lints`) ou configuração explícita por crate? | aberta | — |