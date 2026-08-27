# Dependency update policy

## Dependabot

Dependabot opens bounded weekly proposals for:

- Cargo dependencies at `/`;
- frontend npm dependencies at `/frontend`;
- GitHub Actions at `/`.

Each ecosystem is limited to five open pull requests. Dependabot does not receive
merge authority, approval authority or repository secrets through this configuration.
Every proposal must pass the protected branch required checks before merge.

## Security and review

- `npm audit --audit-level=high` remains fail-closed in the Frontend workflow.
- Rust dependency changes must pass the locked Rust quality matrix.
- Action updates must pass Actionlint, workflow integrity and the applicable CI jobs.
- Breaking upgrades require a focused PR with regression evidence; this policy does
  not authorize `npm audit fix --force` or automatic lockfile rewrites.
- Advisory exceptions require an owner, rationale, expiry and a tracked follow-up.
  The sole current exception is the documented upstream risk for
  `RUSTSEC-2024-0429` / `GHSA-wrw7-89jp-8q8g` in
  `docs/security/advisory-exceptions/RUSTSEC-2024-0429.md`; it is not resolved and
  does not authorize any other advisory.
- The security advisory workflow preserves raw `cargo audit` output, compares findings
  with the resolved package graph, and accepts only that exact reachable exception.
  Any new reachable advisory or audit execution error fails the gate.

## Rollback

Revert `.github/dependabot.yml` and the validation test. Existing dependency pins and
lockfiles are not changed by PR-011.
