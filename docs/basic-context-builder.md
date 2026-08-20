# Basic context builder contract

`agent_runtime::context::basic::BasicContextBuilder` is the first concrete implementation over the provider-neutral context interface. It assembles already-bounded source inputs; it does not query SQLite, retrieve Memory/Skills, invoke providers, execute tools, write memory or render UI prompts.

## Layers and precedence

Accepted layers are Security, System, Project, Agent, Conversation, Task and Tools. They map to the generic source kinds Security, System, Project, Agent, User and Tool. A kind/layer mismatch is omitted as `Disallowed`; a lower layer cannot masquerade as a higher layer.

The generic context contract then applies deterministic precedence and source-ID ordering. Conversation sources retain only the newest `conversation_window` entries in caller order; older entries produce `ConversationWindow` omissions. Task descriptions are agent-trusted metadata, while tool descriptions remain untrusted and `tool_executable=false`.

## Bounds and failure behavior

The builder requires a positive conversation window, preserves the request token budget, propagates cancellation before/during assembly, and preserves explicit budget, duplicate and sensitive omission reasons from the generic builder. Invalid source IDs/content fail in the source contract before assembly. The output contains selected context entries and omission reasons, never raw omitted secret content or executable tool handles.

## Tests

`crates/agent-runtime/tests/basic_context_contract.rs` covers:

- hierarchy assembly and deterministic precedence;
- oldest conversation window omission;
- layer/kind mismatch and metadata-only tools;
- budget, sensitive, duplicate and cancellation propagation;
- invalid window/source fail-closed behavior.

The builder intentionally accepts a source boundary rather than reading Message storage itself; Message storage integration remains an application-service responsibility and cannot introduce unbounded DB reads into this component.

## ONP mapping

- T-376 — Adicionar basic context builder [concluida]