# Spec: Tools core contract

> feature: tools-core
> status: implementada

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

Esta feature implementa PR-096 da queue executável: definir o contrato mínimo e desacoplado para ferramentas executáveis (Tool trait), schema de validação, context de execução, request/response e testes de contrato. O contrato permite que registry, permission engine, runtime e ferramentas concretas interoperem e sejam auditados uniformemente.

## Histórias

### US-601 — Tool trait e contrato verificável

Como desenvolvedor do core, quero um contrato Tool assíncrono, provider-agnostic, com schema validável, context explícito (project/agent/session identity, capability, policy decision, budget, trace) e error taxonomy estruturada, para que ferramentas concretas possam ser adicionadas sem acoplar UI, Tauri, SQLite, providers concretos ou shell irrestrito.

#### AC-601 — Tool trait define execute, can_handle, capabilities, environment @spec:AC-601

- **Dado** uma implementação do trait `Tool`
- **Quando** inspeciono a trait definition
- **Então** a trait define `async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError>`, `fn can_handle(&self, request: &ToolRequest) -> Result<(), ToolError>`, `fn capabilities(&self) -> &[String]`, `fn is_destructive(&self) -> bool`, `fn environment(&self) -> ToolEnvironment`

#### AC-602 — ToolSchema valida input/output JSON schema, capabilities, limits @spec:AC-602

- **Dado** um `ToolSchema`
- **Quando** chamo `schema.validate()`
- **Então** valida name/version não vazios, timeout > 0, payload limits > 0, input_schema/output_schema são JSON objects, retorna Ok para schema válido e Err(ToolSchemaError) para inválido

#### AC-603 — ToolContext transporta project/agent/session, capability, policy, budget, trace @spec:AC-603

- **Dado** um `ToolContext`
- **Quando** chamo `context.validate()`
- **Então** valida capability não vazio e trace_id não vazio, carrega project_id obrigatório, agent_id/session_id/task_id/workflow_id opcionais, capability string, policy_decision (Allow/AskOnce/AskEveryTime/Deny), budget_limits (BudgetLimits), reservation_id opcional, trace_id (TraceId), metadata BTreeMap

#### AC-604 — ToolRequest carrega operation_key, tool_name/version, input, context, timeout @spec:AC-604

- **Dado** um `ToolRequest`
- **Quando** chamo `request.validate()`
- **Então** valida tool_name, tool_version, operation_key não vazios, delega validação do context, retorna Ok para request válido e Err(ToolRequestError) para inválido

#### AC-605 — ToolResponse inclui outcome, payload, trace, duration, metadata @spec:AC-605

- **Dado** um `ToolResponse`
- **Quando** serializo/deserializo via serde
- **Então** mantém operation_key, tool_name, tool_version, outcome (ToolOutcome enum com 10 variants), payload (Value), trace_id, duration_ms, metadata

#### AC-606 — PolicyDecision enum cobre Allow, AskOnce, AskEveryTime, Deny @spec:AC-606

- **Dado** o enum `PolicyDecision`
- **Quando** serializo/deserializo
- **Então** suporta exatamente 4 variants snake_case, roundtrip preserva identidade

#### AC-607 — ToolEnvironment enum cobre Host, Sandbox, Python, Remote @spec:AC-607

- **Dado** o enum `ToolEnvironment`
- **Quando** serializo/deserializo
- **Então** suporta exatamente 4 variants snake_case, roundtrip preserva identidade

#### AC-608 — ToolError taxonomy estruturada sem vazamento de secrets @spec:AC-608

- **Dado** o enum `ToolError`
- **Quando** formatado via Display
- **Então** variantes: NotFound, VersionNotFound, NotActive, CapabilityMismatch, PermissionDenied, ProjectUnauthorized, BudgetExhausted, Timeout, Cancelled, ExecutionFailed, SchemaValidation, Sandbox, Internal; nenhum carrega payload sensível

#### AC-609 — 34 contract tests cobrem validation, serialization, trait behavior @spec:AC-609

- **Dado** a suite de testes `trait_contract.rs`
- **Quando** executo `cargo test -p tool-core`
- **Então** 34 testes passam cobrindo: context validation, request validation, schema validation, trait can_handle (tool name/version/capability/policy), execute success, serialization roundtrip para todos os tipos, Box/Arc trait objects, PolicyDecision/ToolEnvironment variants, ToolError variants

## Fora de escopo

- Implementação de ferramentas concretas (filesystem, terminal, HTTP, Git, Python)
- Tool registry, permission evaluator, sandbox primitive (PR-097..PR-104)
- UI de ferramentas, rendering de tool calls, confirmação humana
- Execução de processos, rede, filesystem fora do contrato

## Suposições

| ID | Suposição | Status | Resolução |
|---|---|---|---|
| ASM-601 | Rust 1.80+ com async-trait 0.1 disponível | confirmada | edition 2024 no tool-core |
| ASM-602 | BudgetLimits e ReservationId já existem em agent-core/budget | confirmada | reutilizados via agent-core::budget |
| ASM-603 | TraceId, OperationKey já existem em agent-protocol/ids | confirmada | OperationKey adicionado via typed_id macro |
| ASM-604 | async_trait crate compatível com edition 2024 | confirmada | verificado em CI |

## Perguntas em aberto

| ID | Pergunta | Status | Resposta |
|---|---|---|---|
| Q-601 | Tool trait deve suportar streaming de output incremental? | respondida | Não neste contrato — streaming de tool output fica fora de PR-096 e pode extender via PR futura sem breaking change do trait básico |
| Q-602 | Schema deve suportar referências JSON Schema ($ref)? | respondida | Não nesta versão — input/output schemas são objetos JSON inline; $ref/remotes ficam fora de escopo por segurança e determinismo |
| Q-603 | Timeout padrão por ambiente ou por tool? | respondida | Por tool via `schema.timeout_seconds`; request pode reduzir via `timeout_seconds` mas nunca exceder o limite do schema |