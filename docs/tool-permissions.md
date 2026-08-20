# Tool permission contract

`ToolPermissionPolicy` is a versioned, provider-independent authorization contract.
Its default effect is always `deny`; explicit rules may produce `allow`, `ask` or
`deny` for Project, Agent or Session scope.

Rules are bounded, scoped by an explicit non-wildcard ID, optionally expire, and may
not contain duplicate conflicting entries. Broad File/Process/Network execution,
invocation or deletion grants are rejected. Unknown fields and unsupported schema
versions fail closed. This module does not execute tools or implement the Permission
Engine; adapters must evaluate the contract before invocation.
