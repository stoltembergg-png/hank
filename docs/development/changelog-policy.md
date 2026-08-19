# Changelog proposal policy

The changelog workflow generates a deterministic proposal from the conventional
commits in the current PR range. It records the source range and exact `HEAD` tip,
categorizes changes, marks breaking changes, and prints an artifact digest.

The workflow does not modify `CHANGELOG.md`, create tags, publish releases, or infer
behavior from comments. A human/release owner must review and apply a proposal in a
future release task. A wrong source range or SHA produces a different proposal and
must not reuse a prior artifact.

Rollback is a revert of the workflow, generator, fixtures and this document.
