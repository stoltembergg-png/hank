# Python tool registration

Python tools enter the existing `tool-core::ToolRegistry` only as validated,
project-scoped declarations. Registration is not execution.

## Required metadata

A declaration must provide:

- valid `ToolSchema` with `environment: python`;
- bounded worker identity;
- project identity and trace ID;
- project origin matching the registration scope;
- capabilities declared in the schema;
- lifecycle/version metadata validated by the existing registry.

Global implicit Python registration is rejected. A declaration from another
project is rejected. Duplicate name/version/scope identities are rejected.

## Execution boundary

The registered handler is a deny-by-default declaration. Resolving metadata
does not execute Python, spawn a process, read environment variables, access the
filesystem or grant capabilities. Execution remains subject to the existing
permission evaluator, lifecycle supervisor and future explicit execution gate.

Descriptions, templates and model/skill text are untrusted metadata and cannot
mutate policy, registry state or authorization.

## Rollback

Unregister/restore uses the existing bounded registry rollback path. It restores
validated metadata only and never runs a handler. Failed validation does not
mutate the registry.
