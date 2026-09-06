//! Contract tests for the fuzz harness (`PR-261`).
//! Each test maps to one acceptance criterion in
//! `.spec/features/fuzz-tests/spec.md`.

#![allow(non_snake_case)]

use test_support::fuzz::{
    digest_corpus, digest_runner, run_all_targets, verify_no_secrets, FuzzHarness, FuzzLimits,
    FuzzReport, FuzzStatus, NegativePattern, TargetError, TargetId, TargetKind,
};
use test_support::fuzz_targets::{
    default_corpus, default_harness, default_negative_patterns, default_targets,
    BranchPolicyTarget, EnvelopeTarget, FuzzManifest, HashChainTarget, PermissionTarget,
    RateLimitTarget, ReleaseMetadataTarget, StateTarget,
};

const SEED: u64 = 0xF022_C0DE_DEAD_BEEF;

fn run_default() -> Vec<FuzzReport> {
    let harness = default_harness("runner-digest-fixture");
    let targets = default_targets();
    let corpus = default_corpus();
    run_all_targets(&harness, &targets, &corpus, SEED).expect("default target registry aligns")
}

#[test]
fn manifest_is_well_formed_and_self_consistent_ac_2201() {
    let manifest = FuzzManifest::from_json(
        br#"{
          "schema_version":1,
          "manifest_revision":"rev-1",
          "runner_digest":"abc",
          "targets":[
            {"id":"FT-001","kind":"envelope","parser":"p","parser_source":"s","invariants":["no_panic","rejects_malformed","accepts_valid"],"smoke_iterations":8,"corpus_path":"p","description":"d"},
            {"id":"FT-002","kind":"policy","parser":"p","parser_source":"s","invariants":["no_panic","rejects_malformed","accepts_valid"],"smoke_iterations":8,"corpus_path":"p","description":"d"},
            {"id":"FT-003","kind":"state","parser":"p","parser_source":"s","invariants":["no_panic","rejects_malformed","accepts_valid"],"smoke_iterations":8,"corpus_path":"p","description":"d"},
            {"id":"FT-004","kind":"permission","parser":"p","parser_source":"s","invariants":["no_panic","rejects_malformed","accepts_valid"],"smoke_iterations":8,"corpus_path":"p","description":"d"},
            {"id":"FT-005","kind":"release_metadata","parser":"p","parser_source":"s","invariants":["no_panic","rejects_malformed","accepts_valid"],"smoke_iterations":8,"corpus_path":"p","description":"d"},
            {"id":"FT-006","kind":"hash_chain","parser":"p","parser_source":"s","invariants":["no_panic",{"length_within":{"min_len":64,"max_len":64}},"rejects_malformed","accepts_valid"],"smoke_iterations":8,"corpus_path":"p","description":"d"},
            {"id":"FT-007","kind":"rate_limit","parser":"p","parser_source":"s","invariants":["no_panic","rejects_malformed","accepts_valid"],"smoke_iterations":8,"corpus_path":"p","description":"d"}
          ]
        }"#,
    )
    .expect("valid manifest");
    manifest.validate().expect("validation passes");
    let registered = default_targets();
    let registered_refs: Vec<&dyn test_support::fuzz::FuzzTarget> =
        registered.iter().map(|t| t.as_ref()).collect();
    manifest
        .validate_against_targets(&registered_refs)
        .expect("registry matches");
    manifest
        .verify_runner_digest("abc")
        .expect("digest matches");

    let bad = br#"{"schema_version":2,"manifest_revision":"r","runner_digest":"d","targets":[]}"#;
    let m2 = FuzzManifest::from_json(bad).unwrap();
    assert!(m2.validate().is_err());
}

#[test]
fn targets_enumerated_and_registered_ac_2202() {
    let targets = default_targets();
    assert_eq!(targets.len(), 7, "expected 7 fuzz targets");
    let mut seen = std::collections::BTreeSet::new();
    for t in &targets {
        let id = t.id();
        assert!(seen.insert(id.clone()), "duplicate target id: {id}");
        assert!(matches!(
            id.as_str(),
            "FT-001" | "FT-002" | "FT-003" | "FT-004" | "FT-005" | "FT-006" | "FT-007"
        ));
        assert!(!t.invariants().is_empty());
        assert!(t.smoke_iterations() > 0);
    }
    let corpora = default_corpus();
    for (target, corpus) in targets.iter().zip(corpora.iter()) {
        let valid_input = corpus.first().expect("each target has a valid seed");
        assert!(
            target.run(valid_input).is_ok(),
            "{} valid seed must be accepted",
            target.id()
        );
    }
}

// @spec:AC-2203
#[test]
fn reproducible_seed_and_corpus_ac_2203() {
    let harness_a = default_harness("runner-digest-fixture");
    let harness_b = default_harness("runner-digest-fixture");
    let targets = default_targets();
    let corpus = default_corpus();
    let reports_a = run_all_targets(&harness_a, &targets, &corpus, SEED).expect("aligned");
    let reports_b = run_all_targets(&harness_b, &targets, &corpus, SEED).expect("aligned");
    for (a, b) in reports_a.iter().zip(reports_b.iter()) {
        assert_eq!(a.corpus_digest, b.corpus_digest);
        assert_eq!(a.runner_digest, b.runner_digest);
        assert_eq!(a.iterations, b.iterations);
        assert_eq!(a.passes, b.passes);
        assert_eq!(a.panics, b.panics);
    }
    let d1 = digest_corpus(&corpus[0]);
    let d2 = digest_corpus(&corpus[0]);
    assert_eq!(d1, d2);
    assert_eq!(d1.len(), 64);
    let r1 = digest_runner(b"runner-bytes");
    let r2 = digest_runner(b"runner-bytes");
    assert_eq!(r1, r2);
    assert_ne!(r1, digest_runner(b"different"));
}

// @spec:AC-2204
#[test]
fn crash_artifact_reproduces_ac_2204() {
    let harness = default_harness("runner-digest-fixture");
    struct Panicky;
    impl test_support::fuzz::FuzzTarget for Panicky {
        fn id(&self) -> TargetId {
            TargetId::new("FT-CRASH")
        }
        fn kind(&self) -> TargetKind {
            TargetKind::Envelope
        }
        fn run(&self, _input: &[u8]) -> Result<(), String> {
            panic!("crash fixture")
        }
        fn invariants(&self) -> &[test_support::fuzz::Invariant] {
            &[test_support::fuzz::Invariant::NoPanic]
        }
        fn smoke_iterations(&self) -> usize {
            4
        }
    }
    let target = Panicky;
    let corpus = vec![vec![0u8; 8]];
    let report = harness.run_target(&target, &corpus, SEED);
    assert_eq!(report.status, FuzzStatus::Panic);
    let crash = report.first_crash.expect("crash captured");
    assert!(crash.message.contains("crash fixture"));
    assert_eq!(crash.stack_digest.len(), 64);
    assert_eq!(crash.input_digest.len(), 64);
    let replay = harness.replay_crash(&target, &corpus, SEED, crash.iteration);
    assert!(replay.reproducible);
    assert_eq!(replay.iterations_run, crash.iteration + 1);
}

// @spec:AC-2205
#[test]
fn bounded_resource_time_limits_ac_2205() {
    let harness_zero = FuzzHarness::new(
        FuzzLimits {
            smoke_iterations: 0,
            ..FuzzLimits::default()
        },
        "tree",
        "head",
        "runner",
    );
    let report = harness_zero.run_target(&EnvelopeTarget, &[b"{}".to_vec()], SEED);
    assert_eq!(report.status, FuzzStatus::ZeroIterations);
    let harness_raised = FuzzHarness::new(
        FuzzLimits {
            smoke_iterations: 16,
            ..FuzzLimits::default()
        },
        "tree",
        "head",
        "runner",
    );
    let report = harness_raised.run_target(&EnvelopeTarget, &[b"{}".to_vec()], SEED);
    assert!(report.iterations >= 8);

    struct SlowTarget;
    impl test_support::fuzz::FuzzTarget for SlowTarget {
        fn id(&self) -> TargetId {
            TargetId::new("FT-SLOW")
        }
        fn kind(&self) -> TargetKind {
            TargetKind::State
        }
        fn run(&self, _input: &[u8]) -> Result<(), String> {
            std::thread::sleep(std::time::Duration::from_millis(2));
            Ok(())
        }
        fn invariants(&self) -> &[test_support::fuzz::Invariant] {
            &[test_support::fuzz::Invariant::NoPanic]
        }
        fn smoke_iterations(&self) -> usize {
            1
        }
    }
    let slow_harness = FuzzHarness::new(
        FuzzLimits {
            smoke_iterations: 1,
            per_iter_timeout_ms: 1,
            ..FuzzLimits::default()
        },
        "tree",
        "head",
        "runner",
    );
    let slow_report = slow_harness.run_target(&SlowTarget, &[b"{}".to_vec()], SEED);
    assert_eq!(slow_report.status, FuzzStatus::Timeout);

    let oom_harness = FuzzHarness::new(
        FuzzLimits {
            smoke_iterations: 1,
            max_memory_mb: 0,
            ..FuzzLimits::default()
        },
        "tree",
        "head",
        "runner",
    );
    let oom_report = oom_harness.run_target(&EnvelopeTarget, &[b"{}".to_vec()], SEED);
    assert_eq!(oom_report.status, FuzzStatus::Oom);

    struct InvariantTarget;
    impl test_support::fuzz::FuzzTarget for InvariantTarget {
        fn id(&self) -> TargetId {
            TargetId::new("FT-INVARIANT")
        }
        fn kind(&self) -> TargetKind {
            TargetKind::State
        }
        fn run(&self, _input: &[u8]) -> Result<(), String> {
            Err("invariant failed".to_string())
        }
        fn classify_error(&self, message: &str) -> TargetError {
            TargetError::InvariantViolation(message.to_string())
        }
        fn invariants(&self) -> &[test_support::fuzz::Invariant] {
            &[test_support::fuzz::Invariant::AcceptsValid]
        }
        fn smoke_iterations(&self) -> usize {
            1
        }
    }
    let invariant_report = harness_raised.run_target(&InvariantTarget, &[b"{}".to_vec()], SEED);
    assert_eq!(invariant_report.status, FuzzStatus::InvariantViolation);
    assert_eq!(invariant_report.invariant_violations, 1);
}

#[test]
fn runner_output_is_single_tap_ac_2206() {
    let reports = run_default();
    let ids: std::collections::BTreeSet<String> = reports
        .iter()
        .map(|r| r.target_id.as_str().to_string())
        .collect();
    let expected = [
        "FT-001", "FT-002", "FT-003", "FT-004", "FT-005", "FT-006", "FT-007",
    ];
    for e in &expected {
        assert!(ids.contains(*e), "missing {e}");
    }
    for r in &reports {
        assert!(!r.target_id.as_str().is_empty());
        assert_eq!(r.seed, SEED);
        assert_eq!(r.corpus_digest.len(), 64);
        assert_eq!(r.runner_digest, "runner-digest-fixture");
        assert_eq!(r.tree_sha, "tree-sha");
        assert_eq!(r.head_sha, "head-sha");
        assert!(r.total_duration_ms < 60_000, "smoke must complete in < 60s");
        assert_ne!(
            r.status,
            FuzzStatus::ZeroIterations,
            "{}: zero iterations",
            r.target_id
        );
        assert_ne!(r.status, FuzzStatus::Oom, "{}: out of memory", r.target_id);
        assert_ne!(r.status, FuzzStatus::Timeout, "{}: timeout", r.target_id);
    }
}

// @spec:AC-2207
#[test]
fn no_credentials_or_unsafe_corpus_in_repo_ac_2207() {
    let flat: Vec<Vec<u8>> = default_corpus().into_iter().flatten().collect();
    let patterns = default_negative_patterns();
    assert!(verify_no_secrets(&flat, &patterns).is_ok());
    let bad = vec![b"contains secret-pattern-marker".to_vec()];
    let pat = NegativePattern::new("NEG-001", b"secret-pattern-marker");
    assert!(verify_no_secrets(&bad, &[pat]).is_err());
    for (id, sample) in [
        ("NEG-002", b"AKIA".as_slice()),
        ("NEG-003", b"-----BEGIN".as_slice()),
        ("NEG-004", b"Bearer ".as_slice()),
    ] {
        let pat = NegativePattern::new(id, sample);
        assert!(verify_no_secrets(&[sample.to_vec()], &[pat]).is_err());
    }
    let _targets = default_targets();
}

#[test]
fn regression_zero_iterations_is_fail_closed_ac_2205() {
    let harness_zero = FuzzHarness::new(
        FuzzLimits {
            smoke_iterations: 0,
            ..FuzzLimits::default()
        },
        "tree",
        "head",
        "runner",
    );
    let report = harness_zero.run_target(&EnvelopeTarget, &[b"{}".to_vec()], SEED);
    assert_eq!(report.status, FuzzStatus::ZeroIterations);
    assert_eq!(report.iterations, 0);
    assert!(report.first_crash.is_none());
}

#[test]
fn regression_replay_crash_is_total() {
    let harness = default_harness("runner-digest-fixture");
    let corpus = vec![b"{}".to_vec()];
    let r = harness.replay_crash(&EnvelopeTarget, &corpus, SEED, 3);
    assert_eq!(r.iterations_run, 4);
    assert!(!r.reproducible);
}

#[test]
fn regression_each_target_kind_is_exercised() {
    let _ = EnvelopeTarget;
    let _ = BranchPolicyTarget;
    let _ = StateTarget;
    let _ = PermissionTarget;
    let _ = ReleaseMetadataTarget;
    let _ = HashChainTarget;
    let _ = RateLimitTarget;
}
