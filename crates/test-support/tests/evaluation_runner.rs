use std::collections::BTreeSet;

use tempfile::tempdir;
use test_support::evaluation::{BaselineReport, EvaluationEffect, EvaluationTerminal};
use test_support::evaluation_corpus::{core_evaluation_corpus, CoreEvaluationFixture};
use test_support::evaluation_runner::{
    NativeEvaluationEnvironment, NativeEvaluationRunner, NativeEvaluationRunnerError,
    MAX_NATIVE_EVALUATION_CASES,
};
use test_support::fixtures::FixtureWorkspace;

fn core_environment(corpus: &[CoreEvaluationFixture]) -> NativeEvaluationEnvironment {
    NativeEvaluationEnvironment::from_evidence(&corpus[0].baseline.evidence).unwrap()
}

fn workspace() -> (tempfile::TempDir, FixtureWorkspace) {
    let directory = tempdir().unwrap();
    let workspace = FixtureWorkspace::create(directory.path().join("native-runner")).unwrap();
    (directory, workspace)
}

// @spec:AC-1453
#[test]
fn core_corpus_replays_to_exact_sha_baseline_reports() {
    let corpus = core_evaluation_corpus().unwrap();
    let environment = core_environment(&corpus);
    let (_directory, fixture_workspace) = workspace();

    let run = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap();

    assert_eq!(run.reports.len(), 6);
    assert_eq!(run.environment, environment);
    assert!(run.reports.iter().all(|report| {
        report.evidence.head_sha == environment.head_sha
            && report.evidence.tree_sha == environment.tree_sha
            && report.evidence.policy_digest == environment.policy_digest
            && report.evidence.schema_digest == environment.schema_digest
            && report.evidence.environment_digest == environment.environment_digest
    }));
    assert_eq!(
        run.reports
            .iter()
            .filter(|report| report.terminal == EvaluationTerminal::Pass)
            .count(),
        4
    );
    assert_eq!(
        run.reports
            .iter()
            .filter(|report| report.terminal == EvaluationTerminal::Blocked)
            .count(),
        1
    );
    assert_eq!(
        run.reports
            .iter()
            .filter(|report| report.terminal == EvaluationTerminal::Cancelled)
            .count(),
        1
    );

    for fixture in &corpus {
        let report = run
            .reports
            .iter()
            .find(|report| report.case_id == fixture.case.case_id)
            .unwrap();
        assert!(report.validate_against(&fixture.case).is_ok());
        assert!(fixture
            .case
            .artifact_requirements
            .iter()
            .all(|requirement| report
                .evidence
                .artifact_digests
                .contains(&requirement.digest)));
    }
}

// @spec:AC-1454
#[test]
fn incomparable_environment_is_rejected_before_materialization() {
    let corpus = core_evaluation_corpus().unwrap();
    let mut environment = core_environment(&corpus);
    environment.head_sha = "sha-different-v1".into();
    let (directory, fixture_workspace) = workspace();

    let error = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        NativeEvaluationRunnerError::IncomparableEnvironment { .. }
    ));
    assert_eq!(
        std::fs::read_dir(directory.path().join("native-runner"))
            .unwrap()
            .count(),
        0
    );
}

// @spec:AC-1454
#[test]
fn fixture_seed_mismatch_is_rejected_before_materialization() {
    let mut corpus = core_evaluation_corpus().unwrap();
    corpus[0].case.fixture.seed += 1;
    let environment = core_environment(&core_evaluation_corpus().unwrap());
    let (directory, fixture_workspace) = workspace();

    let error = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        NativeEvaluationRunnerError::InvalidFixtureBinding { .. }
    ));
    assert_eq!(
        std::fs::read_dir(directory.path().join("native-runner"))
            .unwrap()
            .count(),
        0
    );
}

// @spec:AC-1454
#[test]
fn caller_rewritten_core_identity_is_rejected_before_materialization() {
    let mut corpus = core_evaluation_corpus().unwrap();
    for entry in &mut corpus {
        entry.baseline.evidence.head_sha = "sha-forged-v1".into();
        entry.baseline.evidence.tree_sha = "tree-forged-v1".into();
        entry.baseline.evidence.policy_digest = "policy-forged-v1".into();
        entry.baseline.evidence.schema_digest = "schema-forged-v1".into();
        entry.baseline.evidence.environment_digest = "environment-forged-v1".into();
    }
    let environment =
        NativeEvaluationEnvironment::from_evidence(&corpus[0].baseline.evidence).unwrap();
    let (directory, fixture_workspace) = workspace();

    let error = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        NativeEvaluationRunnerError::IncomparableEnvironment { .. }
    ));
    assert_eq!(
        std::fs::read_dir(directory.path().join("native-runner"))
            .unwrap()
            .count(),
        0
    );
}

// @spec:AC-1455
#[test]
fn missing_artifact_is_rejected_without_replaying_the_fixture() {
    let mut corpus = core_evaluation_corpus().unwrap();
    corpus[0].baseline.evidence.artifact_digests.clear();
    let environment = core_environment(&core_evaluation_corpus().unwrap());
    let (directory, fixture_workspace) = workspace();

    let error = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        NativeEvaluationRunnerError::MissingArtifact { .. }
    ));
    assert_eq!(
        std::fs::read_dir(directory.path().join("native-runner"))
            .unwrap()
            .count(),
        0
    );
}

// @spec:AC-1455
#[test]
fn nondeterministic_fixture_and_external_effect_fail_closed() {
    let mut nondeterministic = core_evaluation_corpus().unwrap();
    nondeterministic[0].case.fixture.deterministic = false;
    let environment = core_environment(&core_evaluation_corpus().unwrap());
    let (_directory, fixture_workspace) = workspace();
    assert!(matches!(
        NativeEvaluationRunner::default().run(&nondeterministic, &environment, &fixture_workspace),
        Err(NativeEvaluationRunnerError::NondeterministicFixture { .. })
    ));

    let mut unsafe_effect = core_evaluation_corpus().unwrap();
    unsafe_effect[0]
        .case
        .allowed_effects
        .push(EvaluationEffect::ExternalWrite);
    let (_directory, fixture_workspace) = workspace();
    assert!(matches!(
        NativeEvaluationRunner::default().run(&unsafe_effect, &environment, &fixture_workspace),
        Err(NativeEvaluationRunnerError::UnsafeEffect { .. })
    ));
}

// @spec:AC-1455
#[test]
fn output_bound_is_checked_before_any_fixture_is_materialized() {
    let mut corpus = core_evaluation_corpus().unwrap();
    corpus[0].case.cancellation.max_output_bytes = 1;
    let environment = core_environment(&core_evaluation_corpus().unwrap());
    let (directory, fixture_workspace) = workspace();

    let error = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        NativeEvaluationRunnerError::OutputBoundExceeded { .. }
    ));
    assert_eq!(
        std::fs::read_dir(directory.path().join("native-runner"))
            .unwrap()
            .count(),
        0
    );
}

// @spec:AC-1455
#[test]
fn later_existing_fixture_conflict_is_rejected_before_earlier_writes() {
    let corpus = core_evaluation_corpus().unwrap();
    let environment = core_environment(&corpus);
    let (directory, fixture_workspace) = workspace();
    let second_fixture_path = fixture_workspace
        .root()
        .join(format!("{}.json", corpus[1].fixture.id));
    let mut tampered_fixture = corpus[1].fixture.clone();
    tampered_fixture.payload = "preexisting tampered fixture".into();
    std::fs::write(
        &second_fixture_path,
        serde_json::to_vec_pretty(&tampered_fixture).unwrap(),
    )
    .unwrap();

    let error = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        NativeEvaluationRunnerError::FixtureContentMismatch { .. }
    ));
    assert!(!fixture_workspace
        .root()
        .join(format!("{}.json", corpus[0].fixture.id))
        .exists());
    assert_eq!(
        std::fs::read_dir(directory.path().join("native-runner"))
            .unwrap()
            .count(),
        1
    );
}

// @spec:AC-1455
#[test]
fn divergent_duplicate_fixture_definitions_are_rejected_before_writes() {
    let mut corpus = core_evaluation_corpus().unwrap();
    let mut duplicate = corpus[1].clone();
    duplicate.fixture.id = corpus[0].fixture.id.clone();
    duplicate.fixture.payload = "synthetic conflicting fixture".into();
    duplicate.case.fixture.fixture_id = duplicate.fixture.id.clone();
    duplicate.case.fixture.fixture_digest = duplicate.fixture.manifest_hash().unwrap();
    duplicate.baseline.evidence.fixture_digest = duplicate.case.fixture.fixture_digest.clone();
    duplicate.baseline = BaselineReport::from_case(
        &duplicate.case,
        duplicate.baseline.terminal,
        duplicate.baseline.metrics.clone(),
        duplicate.baseline.evidence.clone(),
    )
    .unwrap();
    corpus[1] = duplicate;
    let environment = core_environment(&corpus);
    let (directory, fixture_workspace) = workspace();

    let error = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap_err();

    assert!(matches!(
        error,
        NativeEvaluationRunnerError::ConflictingFixtureDefinitions { .. }
    ));
    assert_eq!(
        std::fs::read_dir(directory.path().join("native-runner"))
            .unwrap()
            .count(),
        0
    );
}

// @spec:AC-1456
#[test]
fn replay_is_idempotent_bounded_and_does_not_overwrite_fixtures() {
    let corpus = core_evaluation_corpus().unwrap();
    let environment = core_environment(&corpus);
    let (_directory, fixture_workspace) = workspace();
    let runner = NativeEvaluationRunner::default();

    let first = runner
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap();
    let second = runner
        .run(&corpus, &environment, &fixture_workspace)
        .unwrap();

    assert_eq!(first.run_digest, second.run_digest);
    assert_eq!(
        first
            .reports
            .iter()
            .map(|report| report.report_digest.as_str())
            .collect::<BTreeSet<_>>(),
        second
            .reports
            .iter()
            .map(|report| report.report_digest.as_str())
            .collect::<BTreeSet<_>>()
    );
    assert!(first
        .reports
        .iter()
        .all(|report| serde_json::to_vec(report).unwrap().len() <= 32 * 1024));

    let fixture_path = fixture_workspace
        .root()
        .join(format!("{}.json", corpus[0].fixture.id));
    let mut tampered_fixture = corpus[0].fixture.clone();
    tampered_fixture.payload = "tampered fixture".into();
    std::fs::write(
        &fixture_path,
        serde_json::to_vec_pretty(&tampered_fixture).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        runner.run(&corpus, &environment, &fixture_workspace),
        Err(NativeEvaluationRunnerError::FixtureContentMismatch { .. })
    ));
    let on_disk: test_support::fixtures::FixtureCase =
        serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
    assert_eq!(on_disk.payload, "tampered fixture");

    assert!(NativeEvaluationRunner::new(0, 1).is_err());
    assert!(NativeEvaluationRunner::new(MAX_NATIVE_EVALUATION_CASES + 1, 1).is_err());
    assert!(NativeEvaluationRunner::new(1, 0).is_err());
}
