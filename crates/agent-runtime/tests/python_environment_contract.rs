use agent_core::ids::ProjectId;
use agent_runtime::{
    PythonEnvironmentError, PythonEnvironmentManager, PythonEnvironmentManifest,
    PythonPackageRequirement,
};
use tempfile::tempdir;

fn manifest(project: ProjectId, version: &str) -> PythonEnvironmentManifest {
    PythonEnvironmentManifest::new(
        project,
        "default",
        version,
        vec![PythonPackageRequirement::new(
            "demo",
            "1.0.0",
            "a".repeat(64),
        )],
        vec!["https://pypi.org/simple".into()],
    )
    .unwrap()
}

#[test]
fn manifest_is_sorted_project_scoped_and_rolls_back() {
    let dir = tempdir().unwrap();
    let project = ProjectId::new();
    let manager = PythonEnvironmentManager::new(dir.path());
    let first = manifest(project, "3.11");
    manager.prepare(&first).unwrap();
    assert_eq!(
        manager.load(project, "default").unwrap().python_version,
        "3.11"
    );
    let second = manifest(project, "3.12");
    manager.prepare(&second).unwrap();
    assert_eq!(
        manager.load(project, "default").unwrap().python_version,
        "3.12"
    );
    manager.rollback(project, "default").unwrap();
    assert_eq!(
        manager.load(project, "default").unwrap().python_version,
        "3.11"
    );
}

#[test]
fn invalid_package_source_and_duplicate_fail_closed() {
    let project = ProjectId::new();
    assert!(matches!(
        PythonEnvironmentManifest::new(
            project,
            "../bad",
            "3.11",
            vec![],
            vec!["https://pypi.org/simple".into()]
        ),
        Err(PythonEnvironmentError::InvalidManifest)
    ));
    assert!(PythonEnvironmentManifest::new(
        project,
        "default",
        "3.11",
        vec![PythonPackageRequirement::new("demo", "1.0.0", "bad")],
        vec!["http://pypi.org".into()]
    )
    .is_err());
    let duplicate = vec![
        PythonPackageRequirement::new("demo", "1.0.0", "a".repeat(64)),
        PythonPackageRequirement::new("demo", "1.0.0", "b".repeat(64)),
    ];
    assert!(matches!(
        PythonEnvironmentManifest::new(
            project,
            "default",
            "3.11",
            duplicate,
            vec!["https://pypi.org/simple".into()]
        ),
        Err(PythonEnvironmentError::DuplicatePackage)
    ));
}
