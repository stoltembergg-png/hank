# Fallback policy contract

`provider-core::fallback` is a pure provider-neutral decision engine. It never executes a retry, calls an adapter, changes a credential, or silently switches a model; it returns either one bounded `Retry` attempt or one explicit `Terminal` outcome.

## Retry matrix

Retry is allowed only for:

- rate limited;
- timeout;
- outage;
- quota exceeded.

Authentication, invalid request, policy denied and unsupported failures terminate without fallback. Cancellation also terminates without selecting an alternative.

## Candidate eligibility

A candidate must:

- belong to the same project scope;
- be a different provider from the failed attempt;
- have `HealthStatus::Healthy` evidence;
- have a valid provider/model-bound `CapabilityReport`;
- satisfy every requested capability and limit;
- fit remaining token and cost budgets.

Candidates are sorted deterministically by provider ID, model ID and account ID. Disabled/unhealthy/out-of-scope/incompatible candidates are ignored; if all candidates are blocked, the terminal reason distinguishes budget exhaustion from no eligible alternative.

## Bounds and observability

Policy limits attempts to eight, total tokens to one million and total cost to one billion micro-units. Request IDs and attempt IDs are bounded. Each retry includes a stable logical identity such as `request_1:attempt_2`, preserving stream attempt identity without exposing credential material. The decision includes only provider/model/account metadata and redacted failure class; no raw payload or credential ref is accepted.

## Tests

`crates/provider-core/tests/fallback_contract.rs` covers:

- retryable matrix and deterministic alternative selection;
- auth/invalid-request terminal behavior;
- attempt budget;
- token/cost budget;
- project/capability isolation;
- cancellation;
- attempt identity and bounds;
- debug redaction.

## ONP mapping

- T-369 — Definir política de fallback provider-neutral [concluida]