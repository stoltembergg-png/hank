# Memory policy contract

Memory policy is evaluated before every memory read, write, or learning path.
The policy is project-and-agent scoped, versioned, bounded, and deny-by-default.

## Precedence

Resolution considers layers in this order:

1. `system`
2. `security`
3. `project`
4. `agent`

A lower layer cannot elevate a denial from a higher layer. Missing, foreign, or
invalid identity/policy returns `deny` before content access or mutation.

## Bounds and audit

Policies constrain memory type, requested tokens, cost micros, retention,
approval mode, autonomy, and rollback. The decision contains only `allowed`,
reason, policy version, and layer; raw memory content and secrets are never part
of the policy decision.

The application boundary must supply actor, project, agent, capability and trace
metadata. The model is untrusted input and cannot modify policy or its
precedence. Persistence and application of the resolver are tracked separately
in T-741; this domain slice intentionally has no database or UI dependency.
