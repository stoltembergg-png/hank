use test_support::load::{
    digest_fixture, run_manifest, run_profile, validate_manifest, LoadProfile, LoadStatus,
    WorkloadManifest, CANONICAL_FIXTURE_DIGEST, MAX_PROFILES,
};

// @spec:AC-2301
#[test]
fn manifest_declares_bounded_profiles_and_fixture_digest() {
    let manifest = WorkloadManifest::default();
    assert!(validate_manifest(&manifest));
    assert_eq!(manifest.profiles.len(), MAX_PROFILES);
    assert_eq!(
        manifest.fixture_digest,
        digest_fixture("PR-262:synthetic:redacted")
    );
    assert_eq!(manifest.fixture_digest, CANONICAL_FIXTURE_DIGEST);
}

// @spec:AC-2302
#[test]
fn admission_and_backpressure_are_explicit() {
    let metrics = run_profile(LoadProfile::L, 23_000, "fixture");
    assert_eq!(metrics.status, LoadStatus::AdmissionBound);
    assert_eq!(metrics.admitted + metrics.rejected, metrics.requests);
    assert!(metrics.peak_queue <= LoadProfile::L.limits().queue);
    assert!(metrics.max_in_flight <= LoadProfile::L.limits().concurrency);
}

// @spec:AC-2303
#[test]
fn cancellation_and_completion_are_accounted() {
    let metrics = run_profile(LoadProfile::M, 23_001, "fixture");
    assert_eq!(metrics.completed + metrics.cancelled, metrics.admitted);
    assert!(metrics.bounded_duration_ms <= LoadProfile::M.limits().duration_ms);
    assert!(!metrics.artifact_digest.is_empty());
}

// @spec:AC-2304
#[test]
fn repeated_runs_are_deterministic_and_redacted() {
    let manifest = WorkloadManifest::default();
    let first = run_manifest(&manifest);
    let second = run_manifest(&manifest);
    assert_eq!(first, second);
    assert_eq!(first[0].warmup_iterations, manifest.warmup_iterations);
    assert_eq!(first[0].repetitions, manifest.repetitions);
    let serialized = serde_json::to_string(&first).expect("metrics serialize");
    assert!(!serialized.contains("password"));
    assert!(!serialized.contains("token"));
}

// @spec:AC-2305
#[test]
fn invalid_manifest_fails_closed() {
    let manifest = WorkloadManifest {
        profiles: Vec::new(),
        ..WorkloadManifest::default()
    };
    assert!(!validate_manifest(&manifest));
    assert!(run_manifest(&manifest).is_empty());
}
