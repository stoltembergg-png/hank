# PR title policy

Executable PR titles use either:

```text
PR-###: <summary>
```

or the conventional-commit form:

```text
<type>[optional scope][!]: <summary>
```

The `PR title lint` workflow validates opened, synchronized, reopened and edited
pull requests. It is a deterministic metadata check with read-only permissions;
it does not approve, merge or mutate the PR. Titles are passed as environment data
and never executed as shell input.

Rollback: revert `.github/workflows/pr-title.yml`, the validator, its fixtures and
this policy document.
