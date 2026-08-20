# Budget Policy Contract

`BudgetAccount` and `BudgetHierarchyTracker` establish deterministic, provider-neutral budget accounting and execution resource controls.

## Budget Hierarchy & Precedence

Budget is evaluated in order of hierarchical scopes:
1. **Project Scope:** Highest-level ceiling across all agents, workflows, and sessions within the project.
2. **Agent Scope:** Limits per agent instance to prevent runway consumption by specific delegates.
3. **Session Scope:** Per-session or conversation interaction boundary.
4. **Workflow / Task Scope:** Granular limits on individual orchestrated tasks.

An operation is rejected (`DomainError::BudgetExceeded`) if *any* scope in the hierarchy cannot satisfy the allocation.

## Financial & Accounting Integrity

- **Deterministic Integer Math:** Cost is accounted in integer microdollars (`micro_usd` where $1.00 = 1,000,000 microdollars) to eliminate floating-point non-determinism.
- **Reservation & Commit Semantics:** Long-running or async tasks reserve tokens/cost in advance (`reserve`). Upon completion, the exact consumed amount is committed (`commit`) and remaining unused reservation is released. If the task fails or cancels, the reservation is refunded (`refund`).
- **Periodic Reset:** Accounts support deterministic calendar reset periods (`Daily`, `Weekly`, `Monthly`, or `Never`).
