use agent_core::release_agent_workflow::*;

fn context() -> ReleaseContext {
    ReleaseContext::new("repo", "commit-1", "tree-1", "policy-1").unwrap()
}
fn artifact(commit: &str, digest: &str) -> ArtifactEvidence {
    ArtifactEvidence::new("app.tar", digest, commit, "tree-1", "policy-1").unwrap()
}
fn ci() -> CiEvidence {
    CiEvidence::pass("run-1", "commit-1", "tree-1", "policy-1").unwrap()
}

// @spec:AC-1347
#[test]
fn exact_identity_and_checksum_produce_idempotent_draft_candidate() {
    let input = ReleaseInput::new(
        context(),
        artifact("commit-1", "sha256:abc"),
        ci(),
        true,
        true,
    )
    .unwrap();
    let first = ReleaseAgentWorkflow::prepare(&input).unwrap();
    let second = ReleaseAgentWorkflow::prepare(&input).unwrap();
    assert_eq!(first.state(), ReleaseState::Draft);
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert!(!first.can_publish());
}

// @spec:AC-1347
#[test]
fn wrong_artifact_identity_and_checksum_are_no_go() {
    let input =
        ReleaseInput::new(context(), artifact("other", "sha256:abc"), ci(), true, true).unwrap();
    let result = ReleaseAgentWorkflow::prepare(&input).unwrap();
    assert_eq!(result.state(), ReleaseState::NoGo);
    assert!(!result.reasons().is_empty());
}

// @spec:AC-1348
#[test]
fn missing_signing_or_provenance_is_explicit_no_go() {
    let input = ReleaseInput::new(
        context(),
        artifact("commit-1", "sha256:abc"),
        ci(),
        false,
        true,
    )
    .unwrap();
    let result = ReleaseAgentWorkflow::prepare(&input).unwrap();
    assert_eq!(result.state(), ReleaseState::NoGo);
    assert!(result
        .reasons()
        .iter()
        .any(|reason| reason.contains("signing")));
    assert!(!result.can_publish());
}
