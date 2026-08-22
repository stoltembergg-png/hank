# Python permissions

Python permissions are a specialization of the existing `tool-core`
evaluator, not a second authorization system.

The matrix covers filesystem read/write, network, process and package install.
Every request must carry project identity, declared capability, approval state,
budget state and revocation state.

Default behavior is deny. A request is allowed only when:

- project identity is present and matches the requested project;
- capability is declared;
- approval is valid;
- budget is available;
- policy has not been revoked.

Cross-project requests, undeclared capabilities, missing approval, exhausted
budget and revoked policy produce typed denial reasons. No secret is persisted,
and capability metadata cannot grant authorization by itself.
