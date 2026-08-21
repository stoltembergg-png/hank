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

#### AC-610 — ToolSchema valida semântica de versão e shape @spec:AC-610

- **Dado** um `ToolSchema` com nome/version semânticos, input/output JSON Schema e declarações bounded
- **Quando** chamo `schema.validate()`
- **Então** schemas válidos passam; versão malformada, keyword desconhecida, shape recursivo inválido, capability duplicada/inválida e metadata excedente falham com erro estruturado

#### AC-611 — ToolSchema valida payloads contra limites, tipos e campos obrigatórios @spec:AC-611

- **Dado** um payload de entrada ou saída
- **Quando** chamo `validate_input` ou `validate_output`
- **Então** bytes, profundidade, tipos, `required`, string/array limits, enum e shape são verificados antes do handler; payload excedente falha sem expor conteúdo bruto

#### AC-612 — Política explícita para campos desconhecidos @spec:AC-612

- **Dado** um schema de objeto e payload com campo não declarado
- **Quando** uso `SchemaValidationPolicy::strict()` ou `permissive()`
- **Então** strict rejeita campo desconhecido salvo `additionalProperties` explícito; permissive aceita somente quando o schema não o proíbe; o resultado é determinístico

#### AC-613 — Compatibilidade de versões tem regra explícita @spec:AC-613

- **Dado** um schema na versão semver `1.2.0` e uma versão requisitada
- **Quando** chamo `compatibility_with`
- **Então** a igualdade retorna `Exact`, mesma major retorna `SameMajor`, major diferente retorna `Incompatible` e versão inválida falha fechadamente

#### AC-614 — Schema não aceita campos sensíveis ou instruções executáveis ocultas @spec:AC-614

- **Dado** input/output schema, description ou metadata contendo campo sensível, control character, traversal, command/URL injection ou conteúdo acima do limite
- **Quando** chamo `schema.validate()`
- **Então** a validação rejeita o schema; descrições/examples permanecem dados não confiáveis e nunca alteram capability, ambiente ou policy

#### AC-615 — Contract tests do schema cobrem limites, compatibilidade e isolamento @spec:AC-615

- **Dado** `crates/tool-core/tests/schema_contract.rs`
- **Quando** executo `cargo test -p tool-core --test schema_contract`
- **Então** testes cobrem schema válido/malformado, payload, unknown fields, version compatibility, nested/array constraints, sensitive metadata e ausência de vazamento de conteúdo

#### AC-616 — Registro válido indexa tool sem executar handler @spec:AC-616

- **Dado** um `ToolRegistrationRequest` válido, com schema válido, origem autorizada, scope e trace bounded
- **Quando** chamo `ToolRegistry::register`
- **Então** a tool é indexada por nome/version/scope, lifecycle começa Active e o handler não é executado

#### AC-617 — Registry rejeita duplicidade, schema inválido, origem/scope incompatível e capacidade excedida @spec:AC-617

- **Dado** uma tentativa de registro conflitante ou malformada
- **Quando** chamo `register`
- **Então** retorna erro tipado e não altera o estado anterior; IDs/version/scope são bounded e não atravessam projetos

#### AC-618 — Lookup é determinístico, project-isolated e capability-aware @spec:AC-618

- **Dado** tools globais e project-scoped com nomes/versões iguais ou diferentes
- **Quando** chamo `resolve` ou `list_visible`
- **Então** project scope tem precedência sobre global, projeto errado não resolve, capability ausente/mismatch falha e listagem é determinística

#### AC-619 — Lifecycle impede resolução inativa sem remover metadata @spec:AC-619

- **Dado** uma tool registrada
- **Quando** altero lifecycle para Disabled/Retired e resolvo
- **Então** resolução falha com estado tipado; metadata continua listável e alterações não executam handler

#### AC-620 — Unregister e restore fornecem rollback bounded @spec:AC-620

- **Dado** uma tool registrada
- **Quando** chamo `unregister` e depois `restore`
- **Então** a resolução desaparece e volta com a mesma identidade/scope/origem/lifecycle; duplicidade e restore inválido falham fechadamente

#### AC-621 — Registry suporta seal e concorrência sem estado global @spec:AC-621

- **Dado** operações concorrentes de register/resolve/lifecycle/list
- **Quando** executo em múltiplas threads e selo o registry
- **Então** não há corrida/panic/cross-project leak; leituras continuam possíveis e mutações após seal retornam `Sealed`

#### AC-622 — Registry não confia em descrição/metadata e não executa tools @spec:AC-622

- **Dado** descrições, metadata ou payloads não confiáveis e uma tool fake observável
- **Quando** registro/listo/resolvo
- **Então** somente schema/capability/origin/lifecycle participam da decisão; nenhum conteúdo vira instrução e nenhum handler é chamado

#### AC-623 — Contract tests do registry cobrem dedupe, rollback, lifecycle e isolamento @spec:AC-623

- **Dado** `crates/tool-core/tests/registry_contract.rs`
- **Quando** executo `cargo test -p tool-core --test registry_contract`
- **Então** cobre registro válido, duplicata, lookup existente/inexistente, project scope, capability filter, lifecycle, deterministic listing, unregister/restore, seal, capacity e concorrência

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