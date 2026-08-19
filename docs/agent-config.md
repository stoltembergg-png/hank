# Agent configuration contract

`AgentConfig` is a versioned, domain-only configuration envelope. It requires
`ProjectId` and `AgentId`, uses deterministic defaults, rejects unknown fields and
bounds instruction references/personality content.

Provider selection, credentials, execution and persistence are intentionally excluded.
Future readers may accept compatible schema versions explicitly; unknown versions,
unknown fields, oversized refs and multiline refs fail closed. Configuration changes
are validated before consumers merge them.
