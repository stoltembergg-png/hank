# ADR-HAR-003 — Memory provenance and Evidence Engine

- **Status:** proposed; activates only after PR-270 baseline PASS.
- **Decision:** memory is typed (`working`, `session`, `project`, `long_term`, `skill`, `decision`, `failure`) and every record has project/owner, provenance, authority, retention, version, digest and lifecycle metadata.
- **Decision/failure memory:** decision records reference ADR/SDD/human authority; failure records reference cause, failed approaches, correction, evidence IDs, PR/SHA/version and tags.
- **Evidence:** claims are textual assertions only. A resolver transitions a claim to `VERIFIED`, `UNVERIFIED`, `CONFLICTING`, `STALE`, or `NO_PROOF` using current identity-bound evidence.
- **Consequences:** model output cannot write trusted memory or promote itself to fact; stale evidence is visible, not reused.
- **Rejected:** generic memory bucket, unscoped vector retrieval, raw secret-bearing artifacts, and claim-as-fact promotion.
- **Proof required:** cross-project, poison, duplicate, stale/conflicting evidence, resolver spoof and failure-memory E2E tests.
- **Rollback:** disable candidate activation/retrieval policy; retain records with provenance and tombstone rather than destructive erase where retention permits.
