# Domain error contract

`agent-core::error` exposes stable `DomainErrorCode` and `Retryability` values plus a
serializable `DomainErrorEnvelope`. External boundaries receive the stable code,
retry policy, redacted public message and optional correlation ID; internal payloads
are not exposed.

Validation, authorization, identity and invariant errors are non-retryable. Provider,
tool and workflow failures are conditional. IO failures are safe to retry only when
the owning operation is idempotent. Unknown or malformed errors use a safe fallback
and must not include tokens, prompts, memory contents or raw stacks.

Adding a code requires tests for mapping, retryability, serialization and redaction.
Rollback is a revert of the error contract change; consumers must tolerate unknown
future codes through their versioned fallback.
