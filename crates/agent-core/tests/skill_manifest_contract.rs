use agent_core::{
    Action, Capability, Resource, Skill, SkillFile, SkillFileRole, SkillManifest, SkillScope,
    SkillStatus,
};
use serde_json::json;

fn valid_manifest() -> SkillManifest {
    SkillManifest::new("reviewer", "1.0.0", SkillScope::Project)
}

#[test]
fn valid_manifest_is_bounded_and_roundtrips_without_privileged_fields() {
    let manifest = valid_manifest();
    manifest.validate().expect("fixture must be valid");

    let encoded = serde_json::to_value(&manifest).expect("serialize manifest");
    let decoded: SkillManifest = serde_json::from_value(encoded).expect("deserialize manifest");
    assert_eq!(decoded.name, "reviewer");
    assert_eq!(decoded.version, "1.0.0");
    assert_eq!(decoded.scope, SkillScope::Project);
}

#[test]
fn manifest_rejects_missing_or_invalid_identity() {
    let mut missing_name = valid_manifest();
    missing_name.name.clear();
    assert!(missing_name.validate().is_err());

    let mut invalid_name = valid_manifest();
    invalid_name.name = "../system".into();
    assert!(invalid_name.validate().is_err());

    let mut invalid_version = valid_manifest();
    invalid_version.version = "latest".into();
    assert!(invalid_version.validate().is_err());
}

#[test]
fn manifest_rejects_duplicate_files_capabilities_and_traversal() {
    let mut duplicate_file = valid_manifest();
    duplicate_file.files.push(duplicate_file.files[0].clone());
    assert!(duplicate_file.validate().is_err());

    let mut traversal = valid_manifest();
    traversal.files[0].path = "../outside/SKILL.md".into();
    assert!(traversal.validate().is_err());

    let mut duplicate_capability = valid_manifest();
    let capability = Capability::new(Resource::File, Action::Read);
    duplicate_capability.capabilities = vec![capability.clone(), capability];
    assert!(duplicate_capability.validate().is_err());
}

#[test]
fn manifest_declares_capabilities_without_granting_undeclared_or_secret_access() {
    let mut manifest = valid_manifest();
    let read_file = Capability::new(Resource::File, Action::Read);
    manifest.capabilities.push(read_file.clone());
    manifest
        .validate()
        .expect("read capability is valid metadata");
    assert!(manifest.capability_is_declared(&read_file));
    assert!(!manifest.capability_is_declared(&Capability::new(Resource::File, Action::Update)));

    let mut secret = valid_manifest();
    secret
        .capabilities
        .push(Capability::new(Resource::Secret, Action::Read));
    assert!(secret.validate().is_err());
}

#[test]
fn project_and_global_scope_require_matching_binding() {
    let project = agent_protocol::ids::ProjectId::new();
    let project_skill = Skill::new(valid_manifest(), Some(project));
    project_skill
        .validate()
        .expect("project binding must match");

    let global_skill = Skill::new(
        SkillManifest::new("global-reviewer", "1.0.0", SkillScope::Global),
        None,
    );
    global_skill
        .validate()
        .expect("global skill has no project binding");

    let invalid_global = Skill::new(
        SkillManifest::new("global-reviewer", "1.0.0", SkillScope::Global),
        Some(project),
    );
    assert!(invalid_global.validate().is_err());
}

#[test]
fn lifecycle_matrix_is_explicit_and_archived_is_terminal() {
    assert!(SkillStatus::Draft.can_transition_to(SkillStatus::Testing));
    assert!(!SkillStatus::Draft.can_transition_to(SkillStatus::Active));
    assert!(SkillStatus::Testing.can_transition_to(SkillStatus::Active));
    assert!(SkillStatus::Active.can_transition_to(SkillStatus::Deprecated));
    assert!(SkillStatus::Deprecated.can_transition_to(SkillStatus::Active));
    assert!(!SkillStatus::Archived.can_transition_to(SkillStatus::Draft));

    let mut skill = Skill::new(
        valid_manifest(),
        Some(agent_protocol::ids::ProjectId::new()),
    );
    assert!(skill.activate("1.0.0".into()).is_err());
    skill.transition(SkillStatus::Testing).unwrap();
    skill.activate("1.0.0".into()).unwrap();
    skill.deprecate().unwrap();
    assert_eq!(skill.status, SkillStatus::Deprecated);
}

#[test]
fn unknown_instruction_override_and_sensitive_metadata_are_rejected() {
    let manifest = valid_manifest();
    let mut wire = serde_json::to_value(manifest).expect("serialize fixture");
    wire.as_object_mut()
        .expect("manifest object")
        .insert("instruction_source".into(), json!("system"));
    assert!(serde_json::from_value::<SkillManifest>(wire).is_err());

    let mut override_policy = valid_manifest();
    override_policy.policy.allow_instruction_override = true;
    assert!(override_policy.validate().is_err());

    let mut secret_metadata = valid_manifest();
    secret_metadata.description = "Authorization: Bearer super-secret".into();
    assert!(secret_metadata.validate().is_err());
}

#[test]
fn skill_file_roles_are_declarative_and_do_not_execute() {
    let manifest = valid_manifest();
    assert_eq!(manifest.files[0].role, SkillFileRole::Instruction);
    assert_eq!(manifest.files[0].path, "SKILL.md");
    assert!(!manifest
        .files
        .iter()
        .any(|file: &SkillFile| file.role == SkillFileRole::Script && file.path.contains("run")));
}
