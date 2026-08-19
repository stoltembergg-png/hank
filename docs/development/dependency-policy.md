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
- Advisory exceptions require an owner, rationale, expiry and a tracked follow-up;
  no exception is declared by this configuration.

## Rollback

Revert `.github/dependabot.yml` and the validation test. Existing dependency pins and
lockfiles are not changed by PR-011.
