# Security agent profile

## Boundary

`security-core::security_profile` is a pure, provider-neutral and advisory
contract for threat cases, controls, negative tests, findings and evidence. It
validates project/task/repository/worktree/branch plus exact head/tree/policy
identity and bounded digests.

The profile does **not**:

- exploit production or real users/systems;
- execute payloads, shell, arbitrary commands or provider calls;
- read credentials, secrets or unredacted logs;
- change gates, Rulesets, CODEOWNERS, branches or releases;
- approve, merge or override a human/policy decision.

## Threat manifest

A `SecurityThreatManifest` contains only bounded data IDs (`THREAT-*`, `TM-*`,
`TEST-*`) and a bounded description. The description is untrusted data. Text
such as a prompt injection remains inert metadata; it cannot create a command,
capability or authority.

`SecurityAgentProfile::authorize` requires exact project/task/repository scope,
a current policy revision and an allowlisted control. It returns a permit that
carries identity and threat IDs while all privileged capability queries return
`false`.

## Evidence and findings

`SecurityEvidence` records status, SHA/tree/policy identity, artifact/evidence
digests and bounded artifact bytes. `Passed` and `Failed` evidence requires a
valid artifact digest. `Missing`, `Skipped`, `NoRun`, `Malformed` and `Stale`
are never promoted to success.

Findings explicitly distinguish `Evidence` from `Hypothesis`. A hypothesis is
not proof. A failed evidence item must map to an open evidence finding with the
same threat/control/test IDs and digest; otherwise the report is rejected.

## Reports and handoff

Reports must contain exactly one terminal evidence item for every manifest case.
Wrong identity, duplicate/missing evidence, stale policy, malformed evidence,
missing artifacts and unproven hypotheses fail closed. A report may be `Pass` or
`Fail` only after validation; blockers remain explicit.

A failure/blocker/unknown report can produce a bounded `SecurityHandoff` with
redacted identity and finding IDs. The handoff is advisory and cannot approve,
mutate gates or access secrets.

## Verification boundary

Local contract tests prove the pure domain boundary only. Runtime adapters,
fixture execution, external CI status and production security outcomes require
separate authorized contracts and exact external evidence; this profile never
claims those effects occurred.
