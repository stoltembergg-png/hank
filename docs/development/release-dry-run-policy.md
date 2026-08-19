# Release dry-run policy

PR-016 defines only a release canary. It validates version and SHA/tree identity,
produces an immutable manifest and stops before publication or signing.

The workflow has read-only contents permission, no secrets, no tag mutation and no
publish step. Future signing or publication requires a separate approved increment
with explicit credentials and release policy.
