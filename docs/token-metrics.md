# Token usage metrics contract

PR-095 adds `agent_runtime::usage::UsageEvent` and `UsageAggregator` as a bounded provider-neutral usage ledger. It records only terminal samples and deduplicates by stable `attempt_id`, so duplicate stream/fallback notifications cannot double-count the same attempt. Distinct retry/fallback attempts remain distinct samples because they represent distinct provider work.

## Optionality and confidence

`input_tokens`, `output_tokens` and `cost_micros` remain `Option<u64>`. Missing provider usage is represented by `UsageSource::Missing` and `UsageConfidence::Unavailable`; it is never converted to zero. Provider-reported and estimated samples preserve their source/confidence, and mixed aggregates are explicit `Mixed` rather than silently promoted to exact.

Cost is aggregated only when currency is consistent. A currency mismatch clears the cost amount/currency and sets `currency_mismatch=true`; the read model does not make a billing claim.

## Bounds and isolation

Events require bounded attempt/execution IDs, terminal state, bounded token/cost values and valid currency metadata. The aggregate is bounded by event capacity, uses checked arithmetic and mutates atomically. The aggregate key contains typed Project/Agent/Session identities; read-model lookup is exact and project-scoped. Events contain no prompt, secret, endpoint or raw provider payload.

## UI-ready read model

`frontend/src/chat/usage/UsageSummary.tsx` consumes the normalized optional read model without recalculating metrics. It renders explicit missing/estimated/mixed/currency-mismatch states and is an optional `ChatPage` prop.

## Tests

- `crates/agent-runtime/tests/usage_contract.rs`: reported/missing usage, duplicate attempt, retry/fallback mix, overflow/nonterminal, capacity, invalid currency and redaction.
- `frontend/tests/usage-summary.test.tsx`: reported values, missing usage without fake zeros and currency/confidence/source mismatch states.

## ONP mapping

- T-388 — Adicionar token usage metrics [concluida]