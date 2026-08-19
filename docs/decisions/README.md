# Architecture Decision Records

ADRs record decisions that affect contracts, threat boundaries, persistence, providers
or cross-layer evolution. They are not a place to ratify an unmade product choice.

## Statuses

- `proposed`: alternatives are under review; no implementation authority.
- `accepted`: an explicit decision exists and evidence/rollback are recorded.
- `superseded`: another ADR replaced this decision; link the successor.
- `rejected`: considered and rejected; record why.

Every ADR has stable ID, owner, status, context, alternatives, consequences, risks,
evidence identity and rollback. Secrets, credentials and sensitive dumps are never
stored in ADRs. Capability decisions must include threat boundaries and rollback.

## Creating an ADR

Copy `docs/decisions/ADR-TEMPLATE.md`, assign the next stable ID, add it to
`authority.json`, and run `node tools/adr-lint.mjs`. Do not use `accepted` without an
actual decision and evidence identity. Links are repository-relative and must point
to existing files.
