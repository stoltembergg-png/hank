# AI agent governance

Agents may implement only work selected from the approved SDD/queue and must preserve
non-goals and architecture boundaries. Every external side effect must be scoped,
reversible where possible and verified from the authoritative system.

Agents must:

- work in isolated branches/worktrees;
- use the live Git/GitHub state rather than stale summaries;
- keep generated evidence bound to the exact SHA/tree tested;
- treat mocks as preparation, not external integration proof;
- keep security and required quality gates fail-closed;
- report implemented, locally validated, CI validated and merged as separate states;
- stop for missing credentials, destructive actions, legal/product decisions or
  indispensable unavailable information, after exhausting safe independent work.

Agents must not rewrite history, bypass branch protection, suppress findings, add
secrets, use force dependency repair blindly, or claim a pending/failed check is PASS.
