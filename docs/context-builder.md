# Context builder interface contract

`agent_runtime::context::ContextBuilder` is a deterministic provider-neutral interface for selecting bounded context sources. It does not retrieve Memory/Skills, call providers, render prompts, execute tools or persist snapshots.

## Selection and precedence

Sources are ordered by immutable priority:

```text
Security > System > Project > Agent > User > Provider > Tool > Memory > Skill
```

Equal-priority sources are sorted by source ID. Duplicate keys keep the first higher-priority source and emit an explicit omission. Sensitive sources are omitted explicitly. Missing required source IDs are reported rather than silently invented.

## Trust and tool boundary

User/provider/tool/memory/skill entries are marked `untrusted`; Security/System/Project/Agent entries are not. Tool entries are metadata-only with `tool_executable=false`; the builder has no executor or capability bypass.

Source IDs, content bytes, source count and token budget are bounded. Secret-like content, controls and invalid requests fail closed. Cancellation is checked before and during selection. Output includes selected entries, omissions/reasons and consumed token estimate without hidden sources.

## Tests

`crates/agent-runtime/tests/context_contract.rs` covers:

- deterministic priority and budget omissions;
- duplicate/sensitive omission;
- required missing sources and untrusted marking;
- tool metadata-only behavior;
- cancellation;
- bounds and secret-like content rejection;
- bounded output/redacted omission metadata.

## ONP mapping

- T-375 — Adicionar context builder interface [concluida]