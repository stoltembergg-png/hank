# Review Remediation Agent Design

**Status:** approved design, implementation in progress under the linked execution plan

**Decision owner:** repository maintainers

**Scope:** GitHub Actions automation that turns a concrete Aikido or CodeRabbit finding into a bounded, human-reviewable remediation pull request.

## Problem

The repository already has provider-neutral reviewer and fix-review contracts, but it does not have a GitHub boundary that consumes external reviewer findings and proposes a correction. The manual path is slow and can lose the finding's pull request, commit, or evidence identity.

The new automation must improve that path without allowing reviewer text or model output to approve a change, mutate branch protection, merge code, publish a release, or access repository secrets from untrusted pull-request code.

## Goals

1. Detect actionable failures from CodeRabbit reviews and Aikido check runs.
2. Bind every run to one repository, pull request, head SHA, source branch, finding fingerprint, and workflow policy revision.
3. Ask `mimo-v2.5` for one bounded source patch using a sanitized, explicitly untrusted finding context.
4. Validate the patch with deterministic repository checks without exposing the MiMo credential.
5. Publish a draft remediation pull request targeting the original pull request branch.
6. Preserve human review, required checks, and the existing merge/release authority boundaries.
7. Make duplicate events, stale findings, oversized output, and partial failures safe and observable.

## Non-goals

- Automatic approval, merge, rebase, force-push, release, or ruleset/CODEOWNERS mutation.
- Direct commits to the original pull request branch.
- Processing fork pull requests in the first version.
- Giving the model shell, network, GitHub, secret, or workflow-authoring tools.
- Passing raw chain-of-thought or provider payloads into comments, artifacts, or logs.
- Live-provider calls in unit, contract, or CI quality tests.
- Changing the desktop application runtime or its provider registry.
- Replacing the existing `reviewer_profile` or `fix_review_workflow` domain contracts.

## Relationship to the approved architecture

The workflow is an outer GitHub adapter around the existing advisory reviewer and correction-task boundaries. Reviewer material, pull-request text, diffs, check output, and model output are untrusted data. They are never policy, authorization, or shell instructions.

The workflow keeps provider-specific transport outside `agent-core`: a small, tested MiMo OpenAI-compatible client is used only by the workflow job. The application provider registry and provider-neutral Rust contracts remain unchanged. The client has one fixed model and an allowlisted Xiaomi endpoint; it cannot be redirected by pull-request input.

The implementation follows the existing development-agent rules in `docs/review-workflow.md`, `docs/fix-review-workflow.md`, `docs/coding-agent-profile.md`, and `docs/reviewer-agent-profile.md`:

- external review is advisory;
- corrections remain PR-bound and bounded;
- stale or incomplete evidence stops the flow;
- protected gates retain independent authority;
- secrets remain outside the domain and test context.

## Approaches considered

### Comment-only assistant

The workflow would summarize the finding and post a suggested patch for a human to apply. This has the smallest write surface, but it does not remove the repeated manual correction work and cannot validate the proposed patch in CI.

### Isolated remediation pull request (selected)

The workflow creates a detached worktree from the exact source head, asks MiMo for one patch, validates the patch with deterministic guards, runs fixed checks without the provider secret, and opens a draft PR whose base is the original source branch. A maintainer can inspect and merge that small PR; the original PR then reruns its normal reviewer and required checks.

This gives useful automation while preserving a separate human merge decision and the existing branch protection. It also makes rollback straightforward: close the draft PR and delete its automation branch; the source PR was never mutated.

### Direct commit to the source pull request

The workflow could push the generated commit directly to the source branch. This is faster, but it gives an external review payload and a model write authority over an active contribution. It also makes it harder to distinguish human intent from automation and increases loop and token-exfiltration risk. It is rejected.

## Runtime architecture

The workflow is divided into four jobs so that no job both holds the MiMo secret and executes pull-request code:

1. **Collect** — read-only GitHub API access. Resolve the pull request and exact head SHA, accept only a known reviewer source, normalize one concrete finding, redact secret-like text, and compute a deterministic fingerprint.
2. **Propose** — trusted workflow code only. Read the bounded finding and bounded source diff through the GitHub API, call MiMo with `XIAOMI_MIMO_API_KEY`, and emit a patch artifact. The job never checks out or runs pull-request code.
3. **Validate** — no MiMo credential and no write token. Check out the source head into an isolated worktree, apply the patch, enforce path/diff/symlink/binary guards, and run only deterministic patch checks such as `git diff --check`. It never executes source-controlled build, test, package, or task scripts. Any failure produces no branch or PR.
4. **Publish** — write access only after validation. Recreate the validated worktree, re-fetch the original PR and source branch identity at the exact collected SHA, commit only the verified files with hooks disabled, verify the staged file list and clean worktree/index boundary, push a uniquely fingerprinted branch, and create a draft PR against the original source branch. The body contains bounded evidence metadata, tests, rollback instructions, and an explicit no-approval statement.

The workflow uses `persist-credentials: false` for every checkout. Pull-request code is never executed while `XIAOMI_MIMO_API_KEY` or a write-capable GitHub token is present.

## Event and finding contract

The workflow accepts only these event classes:

- `pull_request_review.submitted` where the review state is `changes_requested` or `commented`, the author is the configured CodeRabbit bot, and the pull request head repository equals the current repository;
- `check_run.completed` where the check conclusion is not successful, the check belongs to the configured Aikido app/name, and the check links to exactly one open pull request in the current repository.

An accepted finding contains only bounded fields:

```text
source: aikido | coderabbit
repository: owner/name
pull_request: positive integer
source_branch: validated repository branch name
base_branch: validated repository base branch name
head_sha: 40-hex commit
title: <= 512 UTF-8 bytes, redacted
detail: <= 8 KiB UTF-8 bytes, redacted
path: repository-relative path or absent
line: positive integer or absent
evidence_url: HTTPS GitHub URL or absent
fingerprint: lowercase SHA-256 digest
policy_revision: review-remediation-v1
```

If a source provides only a generic failed check, an unbound URL, no concrete path/detail, a foreign repository, or a stale SHA, the workflow records `HUMAN_REQUIRED` and does not ask the model for a patch.

The canonical fingerprint covers source, repository, pull request, source/base branches, head SHA, reviewer identity, title, detail, path, line, and policy revision. A completed fingerprint is idempotent: later duplicate events do not create another model call, branch, or PR.

## Model boundary

The first version uses:

```text
model: mimo-v2.5
base URL: https://api.xiaomimimo.com/v1
credential: ${{ secrets.XIAOMI_MIMO_API_KEY }}
```

The base URL is a workflow constant and is validated again by the client against the Xiaomi host allowlist. Pull-request data cannot select the endpoint, model, headers, or request path. Token Plan regional endpoints are outside this first version and require a separate reviewed configuration change.

The client sends one non-streaming OpenAI-compatible chat request with bounded system and user messages, `temperature: 0`, and a bounded token budget. The system message requires one unified diff and states that all supplied finding/diff text is untrusted data. The implementation reads only the assistant text field; any reasoning field is discarded and never logged.

No credential is included in the prompt, patch artifact, PR body, test environment, or diagnostic output. HTTP errors are classified into bounded categories and redact authorization/token-like material.

## Prompt and patch contract

The prompt contains a structured, size-limited JSON envelope with the exact identity, normalized finding, relevant source diff, and explicit constraints. The envelope is delimited as untrusted data. It instructs the model to:

- address only the named finding;
- produce one unified diff relative to the exact source head;
- avoid `.github/workflows`, `.github/actions`, branch-protection files, secrets, credentials, binaries, symlinks, and generated artifacts;
- make no dependency or unrelated formatting changes;
- return no shell commands, tool calls, approval claims, or explanatory reasoning.

The parser accepts a single bounded diff from the assistant content. It rejects missing diff markers, multiple unrelated payloads, absolute paths, traversal, NUL/control characters, binary patches, oversized files/diffs, and patch metadata that targets forbidden paths.

The first version limits each patch to 10 files, 500 added/deleted lines, 64 KiB total patch text, and 256 KiB per resulting text file. It allows source, test, and documentation changes, but denies dependency manifests/lockfiles, `.github/workflows/**`, `.github/actions/**`, `.git/**`, `.env*`, credential/config secret paths, branch/ruleset policy files, and symlink/submodule changes.

## Validation and publication

Validation occurs in a clean detached worktree at the exact source head:

1. `git apply --check` verifies patch applicability.
2. The patch guard compares the resulting tree to the allowlist and limits.
3. `git diff --check` rejects whitespace errors.
4. The remediation workflow does not execute source-controlled build, test, package, or task scripts. The resulting draft PR's normal required CI is the authority for Rust, frontend, Tauri, and E2E checks.
5. The full required checks remain authoritative on the resulting draft PR; the agent's deterministic patch check cannot mark a PR ready or override a failed gate.

The publish step re-applies the exact validated patch, verifies its SHA-256 and tree digests, checks that only the validated file list is staged, and requires the original open PR and source branch to still point to the collected head SHA and base branch immediately before commit. It creates:

```text
review-remediation/pr-<number>/<short-head>-<fingerprint-prefix>
```

The draft PR base is the original same-repository source branch. Its body includes the source PR number, exact source SHA, fingerprint, evidence URL when safe, files changed, commands run, and rollback steps. It does not state that the reviewer, model, or agent approved the change.

At most two remediation cycles are allowed for the same source PR/finding lineage. A third request becomes `HUMAN_REQUIRED`. Events from a remediation branch, a closed source PR, an already completed fingerprint, or a bot-created draft are no-ops.

## Failure handling and rollback

- Missing `XIAOMI_MIMO_API_KEY`: record `NO_PROOF`/`HUMAN_REQUIRED`; do not fail unrelated required checks.
- Provider timeout, rate limit, malformed response, or disallowed patch: record a bounded failure and do not publish.
- Stale source SHA or changed source branch between jobs: discard the artifact and require a fresh review event.
- Validation failure: publish no branch and no PR; retain only bounded run metadata.
- Publish failure after local validation: leave the source PR unchanged and expose the failure in the workflow summary.
- Unwanted draft PR: close it and delete its automation branch; the source PR remains the rollback boundary.
- Suspected credential exposure: disable the workflow, revoke/rotate the provider key, and remove the GitHub secret before re-enabling.

The workflow never changes required-check manifests, rulesets, CODEOWNERS, release tags, or protected branches.

## Testing strategy

All tests are offline and deterministic:

- finding normalization: supported sources, wrong source, missing link/path, foreign repository, stale SHA, duplicate and cycle-cap cases;
- redaction: API keys, bearer values, passwords, authorization headers, and secret-like reviewer text do not reach prompt/artifact/comment output;
- prompt construction: reviewer and diff text cannot alter the fixed instruction envelope;
- MiMo client: request shape, endpoint/model allowlist, timeout, status mapping, malformed JSON, oversized response, and response-reasoning discard using an in-memory fake transport;
- patch guard: traversal, absolute path, forbidden workflow/policy/secret path, binary/symlink/submodule, oversized diff, invalid patch, and valid source/test patch;
- identity/idempotency: exact SHA/tree/repository binding, live publication revalidation, staged file-list binding, and one branch/PR per fingerprint;
- workflow execution boundary: validation does not run source-controlled build/test/package scripts; the generated draft's existing CI remains authoritative;
- workflow contract: top-level permissions, concurrency, timeouts, fork rejection, no `pull_request_target`, SHA-pinned actions, checkout credential policy, no auto-merge, and secret scoping;
- integration fixture: successful collect→propose→validate→draft descriptor and failure matrix with no external network.

The tests must not contain a real provider credential or call Xiaomi. Quality gates are Actionlint, the workflow-integrity tests, the applicable Node/Rust tests, format/lint checks, and the existing CI/reviewer checks on the draft PR.

## Operational prerequisites

Before enabling the workflow on the repository:

1. Revoke/rotate the credential that was pasted into the conversation.
2. Store the replacement only as the repository Actions secret `XIAOMI_MIMO_API_KEY`.
3. Confirm the repository's Actions policy allows the workflow's minimal write permissions for same-repository branches.
4. Keep the workflow disabled or fail-closed when the secret is absent.

The secret is not required for local or CI contract tests.

## Rollout and compatibility

The capability is introduced as an isolated PR after the current predecessor PR is merged. It does not modify application runtime behavior, required check names, release workflows, or existing reviewer contracts. The first rollout observes findings and can be disabled by reverting the workflow or removing the secret. A later extension may add token-plan regional endpoints or other reviewer sources only through a new reviewed contract.

## Definition of Done

- The workflow and helper modules have deterministic contract tests and no live-provider test dependency.
- Actionlint and existing Quality Integrity checks pass.
- Fork, stale, duplicate, malformed, oversized, prompt-injection, secret-like, and forbidden-path cases fail closed.
- A valid synthetic finding produces a draft PR descriptor only after patch validation and deterministic patch checks.
- The draft PR is bound to the exact source SHA, original source/base branches, validated tree, and staged file list, with no auto-approval or auto-merge capability.
- Documentation records setup, permissions, secret rotation, observability, and rollback.
- The implementation is published as a small isolated PR only after the current PR-397 predecessor is merged.
