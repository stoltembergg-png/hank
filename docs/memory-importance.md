# Memory importance scoring

`MemoryImportanceScorer` computes a bounded, deterministic score from metadata
only:

```text
confidence × 0.6
+ recency factor × 0.2
+ repetition factor × 0.2
```

The result includes policy version, trace ID, factors and a threshold-derived
`eligible` flag. The text content is explicitly excluded from both score and
explanation, so claims such as `importance=1.0`, prompt injection or secrets
cannot manipulate the result.

Invalid policy, missing trace identity and invalid confidence fail closed. A
score is advisory metadata; it does not approve, persist or activate memory.
