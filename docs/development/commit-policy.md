# Conventional commit policy

Commits in executable PRs use:

```text
<type>[optional scope][!]: <imperative subject>
```

Allowed types are `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`,
`refactor`, `revert`, `style` and `test`. Subjects are non-empty and limited to
72 characters. Breaking changes use `!` or a `BREAKING CHANGE:` footer.

The `Commit lint` workflow validates the PR commit range and runs fixtures. It
reads commit subjects as data; it never executes them as shell input. Merge commits
created by GitHub are accepted as transport history, while authored commits remain
subject to the policy.

Rollback: revert `.commitlintrc.json`, `tools/commit-message-lint.mjs`, its fixtures,
the workflow and this document. No author identity or historical commit is rewritten.
