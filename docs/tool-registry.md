# Tool registry contract (PR-098)

`tool_core::ToolRegistry` is an in-process, bounded registry for validated executable tool contracts. It indexes `(name, version, scope)` in a `BTreeMap` protected by one `RwLock`, so mutation and read behavior are deterministic and thread-safe without global mutable state.

## Registration

`ToolRegistrationRequest` carries the `Arc<dyn Tool>`, authorized origin, visibility scope and trace ID. Registration:

1. validates the tool's `ToolSchema`;
2. checks origin/scope binding (`Builtin` is global, `Project(id)` must match `Project(id)`, trusted extensions are bounded);
3. rejects duplicate identity and capacity overflow;
4. records lifecycle as `Active`;
5. never invokes the handler.

Descriptions and metadata are not used as instructions or policy inputs. Registry descriptors expose only normalized identity, origin, scope, lifecycle, capabilities, destructive flag and environment.

## Resolution and isolation

`ToolLookupRequest` requires name, version, project ID and trace ID, with an optional capability requirement. Resolution checks the project-scoped key first, then the global key. A project registration never becomes visible to another project. A project-specific inactive registration blocks fallback for that identity instead of silently bypassing lifecycle.

`list_visible` returns global plus same-project descriptors in deterministic key order. `list_by_capability` applies an exact declared capability filter and returns normalized descriptors only.

## Lifecycle and rollback

`Active` tools resolve. `Disabled` and `Retired` tools remain in metadata listings but resolve with a typed `NotActive` error. `unregister` returns a bounded `RemovedTool` handle; `restore` re-registers the same tool identity/origin/scope/lifecycle. The registry can be sealed; after sealing, registration, lifecycle mutation, unregister and restore fail with `Sealed`, while reads remain available.

## Security and non-goals

The registry does not execute handlers, evaluate final permissions, access filesystem/network, discover remote tools, load code implicitly or store secrets. Schema validation and capability declarations are prerequisites; permission evaluation is PR-099. All error values contain bounded identities or stable categories, never raw payloads.

## Tests

`crates/tool-core/tests/registry_contract.rs` has 10 contract tests covering registration/no execution, duplicate/schema/origin/capacity rejection, project precedence/isolation, capability mismatch, lifecycle, unregister/restore, sealing, deterministic listing, capability filter and concurrent access.