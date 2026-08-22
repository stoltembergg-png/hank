# Test & Verification Platform — Adversarial Review

## Surviving hard constraints

1. **No false PASS:** skipped, unavailable credential, stale artifact, quarantined test, mock-only result or LLM judge opinion cannot satisfy a required integration/release claim.
2. **No shared test state:** every fixture owns temp paths, ports, IDs, clock, artifacts and cleanup; tests must run independently and in safe parallelism.
3. **No browser-to-desktop conflation:** Playwright browser E2E is a webview/application-surface check. Native Tauri window/IPC/permission evidence requires a separate driver and OS matrix.
4. **No real external mutation in PR CI:** protected Telegram/GitHub/provider suites use dedicated fixtures and least privilege; unavailable credentials are `NO_PROOF`.
5. **No security by CodeQL alone:** threat corpus, permission/path/injection/secret/evidence negative tests remain first-class gates.
6. **No slow everything-on-every-PR:** deterministic FAST/CORE checks run on PRs; expensive fuzz/load/soak/chaos/full OS/external suites run nightly or release according to explicit policy.
7. **No release by artifact existence:** clean install, artifact identity and upgrade/rollback evidence are mandatory before strong release claims.
8. **No flaky deletion:** quarantine is explicit, time-bounded, owned and never silently allows critical security/recovery tests to pass.

## Risks and planning mitigations

| Risk | Mitigation card(s) |
|---|---|
| Tests merely repeat implementation | PR-346 catalog + PR-352 real boundary matrix + golden E2Es |
| Excessive mocks | PR-349 MockProvider only for deterministic behavior; PR-351 real SQLite; PR-367 protected real integrations |
| Flaky browser/desktop suites | PR-348 deterministic clock/IDs; PR-356 artifacts; PR-358 capability gate; PR-375 quarantine policy |
| Desktop cross-platform overclaim | PR-358–360 require per-OS evidence or `NO_PROOF` |
| Release without install evidence | PR-370–372 clean install/upgrade/rollback/rehearsal |
| Recovery untested | PR-362 crash/event/scheduler recovery E2E and PR-366 chaos |
| Secret leakage in CI artifacts | PR-353 corpus, PR-367 policy, artifact redaction requirements |
| LLM behavior judged only by LLM | PR-349 deterministic MockProvider, PR-369 structured eval assertions, real-model smoke separate |
| Change selection hides regression | PR-375 keeps release full matrix mandatory |

## Rejected shortcuts

- One giant workflow that serializes all tests.
- `continue-on-error` or retries as a substitute for diagnosis.
- Browser E2E presented as native desktop E2E.
- Automatic test quarantine.
- Real provider/Telegram/GitHub credentials on PR forks.
- Line coverage threshold as a substitute for behavioral/security coverage.
- Fuzz/load/soak on every PR without risk or cost justification.

## Review conclusion

The queue is defensible only if it starts after PR-345, preserves PR-001..PR-345 integrity, uses deterministic foundations before costly E2E, keeps protected integrations separate, and fails closed for every missing proof. Otherwise the platform would create slower CI without increasing trust.
