//! SQLite repository for immutable Skill versions and scoped mutable heads.
//!
//! The repository stores parsed content as untrusted data only. It never
//! resolves references, imports global Skills implicitly, changes runtime
//! state, or executes an artifact.

use crate::event_bus::EventBus;
use agent_core::{
    DomainError, ParsedSkill, ProjectId, Skill, SkillCompatibility, SkillError, SkillId,
    SkillScope, SkillStatus,
};
use agent_protocol::events::{ApplicationEvent, EventKind, GlobalApplicationEvent};
use agent_protocol::ids::EventId;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct SkillRecord {
    pub skill: Skill,
    pub parsed: ParsedSkill,
    pub revision: u64,
    pub version_id: String,
    pub content_hash: String,
    pub parent_version: Option<String>,
    pub compatibility: SkillCompatibility,
}

#[derive(Clone)]
pub struct SqliteSkillRepository {
    pool: Pool<Sqlite>,
    event_bus: Option<EventBus<ApplicationEvent>>,
    global_event_bus: Option<EventBus<GlobalApplicationEvent>>,
}

impl SqliteSkillRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self {
            pool,
            event_bus: None,
            global_event_bus: None,
        }
    }

    pub fn with_event_bus(mut self, event_bus: EventBus<ApplicationEvent>) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    pub fn with_global_event_bus(mut self, event_bus: EventBus<GlobalApplicationEvent>) -> Self {
        self.global_event_bus = Some(event_bus);
        self
    }

    pub async fn create(
        &self,
        skill: &Skill,
        parsed: &ParsedSkill,
    ) -> Result<SkillRecord, DomainError> {
        validate_input(skill, parsed)?;
        let content_hash = compute_content_hash(skill, parsed)?;
        let namespace = namespace(skill.manifest.scope, skill.project_id.as_ref())?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| persistence_error("skill create transaction", error))?;

        insert_version(
            &mut transaction,
            &namespace,
            skill,
            parsed,
            None,
            SkillCompatibility::Initial,
            &content_hash,
        )
        .await?;
        insert_version_state(
            &mut transaction,
            &namespace,
            skill,
            status_to_db(skill.status),
        )
        .await?;
        let head_result = sqlx::query(
            "INSERT INTO skill_heads (namespace, skill_id, project_id, scope, current_version, status, pinned_version, activated_at, rollback_version, revision, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?)",
        )
        .bind(&namespace)
        .bind(skill.manifest.id.to_string())
        .bind(skill.project_id.map(|id| id.to_string()))
        .bind(scope_to_db(skill.manifest.scope))
        .bind(&skill.manifest.version)
        .bind(status_to_db(skill.status))
        .bind(skill.pinned_version.as_deref())
        .bind(skill.activated_at.map(|value| value.to_rfc3339()))
        .bind(skill.rollback_version.as_deref())
        .bind(Utc::now().to_rfc3339())
        .execute(&mut *transaction)
        .await;
        match head_result {
            Ok(_) => {}
            Err(error) if is_unique_violation(&error) => {
                return Err(DomainError::Duplicate(
                    "skill identity or version already exists".into(),
                ));
            }
            Err(error) => return Err(persistence_error("skill head create", error)),
        }
        transaction
            .commit()
            .await
            .map_err(|error| persistence_error("skill create commit", error))?;
        let record = SkillRecord {
            skill: skill.clone(),
            parsed: parsed.clone(),
            revision: 1,
            version_id: version_id(&skill.manifest.id, &skill.manifest.version),
            content_hash,
            parent_version: None,
            compatibility: SkillCompatibility::Initial,
        };
        self.publish_version_event("create", &record, skill.project_id);
        Ok(record)
    }

    pub async fn get(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        skill_id: &SkillId,
    ) -> Result<Option<SkillRecord>, DomainError> {
        let namespace = namespace(scope, project_id)?;
        self.get_current_by_namespace(&namespace, skill_id).await
    }

    pub async fn get_version(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        skill_id: &SkillId,
        version: &str,
    ) -> Result<Option<SkillRecord>, DomainError> {
        let namespace = namespace(scope, project_id)?;
        let row = sqlx::query(
            "SELECT sv.skill_json, sv.parsed_json, sv.content_hash, sv.parent_version, sv.compatibility, sv.hash_algorithm, states.status AS release_status FROM skill_versions AS sv LEFT JOIN skill_version_states AS states ON states.namespace = sv.namespace AND states.skill_id = sv.skill_id AND states.manifest_version = sv.manifest_version WHERE sv.namespace = ? AND sv.skill_id = ? AND sv.manifest_version = ?",
        )
        .bind(&namespace)
        .bind(skill_id.to_string())
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| persistence_error("skill version query", error))?;
        row.map(|row| decode_version(&row, 0)).transpose()
    }

    pub async fn list(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<SkillRecord>, DomainError> {
        let namespace = namespace(scope, project_id)?;
        let rows = sqlx::query(
            "SELECT skill_id, revision FROM skill_heads WHERE namespace = ? ORDER BY updated_at DESC, skill_id ASC LIMIT ? OFFSET ?",
        )
        .bind(&namespace)
        .bind(limit.min(100) as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| persistence_error("skill list", error))?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let skill_id = parse_skill_id(&row_string(&row, "skill_id")?)?;
            if let Some(mut record) = self.get_current_by_namespace(&namespace, &skill_id).await? {
                record.revision = nonnegative_revision(row_i64(&row, "revision")?)?;
                records.push(record);
            }
        }
        Ok(records)
    }

    pub async fn list_versions(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        skill_id: &SkillId,
    ) -> Result<Vec<SkillRecord>, DomainError> {
        let namespace = namespace(scope, project_id)?;
        let rows = sqlx::query(
            "SELECT sv.skill_json, sv.parsed_json, sv.content_hash, sv.parent_version, sv.compatibility, sv.hash_algorithm, states.status AS release_status FROM skill_versions AS sv LEFT JOIN skill_version_states AS states ON states.namespace = sv.namespace AND states.skill_id = sv.skill_id AND states.manifest_version = sv.manifest_version WHERE sv.namespace = ? AND sv.skill_id = ? ORDER BY sv.created_at ASC, sv.manifest_version ASC",
        )
        .bind(namespace)
        .bind(skill_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| persistence_error("skill versions list", error))?;
        rows.iter().map(|row| decode_version(row, 0)).collect()
    }

    pub async fn update(
        &self,
        skill: &Skill,
        parsed: &ParsedSkill,
        expected_revision: u64,
    ) -> Result<SkillRecord, DomainError> {
        let namespace = namespace(skill.manifest.scope, skill.project_id.as_ref())?;
        let expected = revision_i64(expected_revision)?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| persistence_error("skill update transaction", error))?;
        let head = head_in_transaction(&mut transaction, &namespace, &skill.manifest.id).await?;
        let Some(head) = head else {
            return Err(DomainError::NotFound("skill head not found".into()));
        };
        if head.revision != expected_revision {
            return Err(concurrency_error(expected_revision, head.revision));
        }
        if skill.status == SkillStatus::Active {
            return Err(DomainError::InvalidStateTransition {
                from: "active".into(),
                to: "draft/testing; use promote".into(),
            });
        }
        if skill.pinned_version.is_some() {
            return Err(DomainError::Validation(
                "skill update cannot set an activation pin".into(),
            ));
        }
        if head.current_version == skill.manifest.version {
            return Err(DomainError::Duplicate(
                "skill manifest version is immutable".into(),
            ));
        }
        let compatibility =
            SkillCompatibility::from_parent(Some(&head.current_version), &skill.manifest.version)
                .map_err(|error| DomainError::Validation(error.to_string()))?;
        let candidate_hash = compute_content_hash(skill, parsed)?;
        if skill.status == SkillStatus::Active && compatibility == SkillCompatibility::Incompatible
        {
            return Err(DomainError::PermissionDenied {
                capability: "skill.activate".into(),
                reason: "incompatible skill version requires an explicit compatibility decision"
                    .into(),
            });
        }
        if let Some(mut existing) = version_by_content_in_transaction(
            &mut transaction,
            &namespace,
            &skill.manifest.id,
            &candidate_hash,
        )
        .await?
        {
            existing.revision = head.revision;
            return Ok(existing);
        }
        let mut persisted = skill.clone();
        persisted.parent_version = Some(head.current_version.clone());
        persisted.compatibility = compatibility;
        validate_input(&persisted, parsed)?;
        insert_version(
            &mut transaction,
            &namespace,
            &persisted,
            parsed,
            persisted.parent_version.as_deref(),
            compatibility,
            &candidate_hash,
        )
        .await?;
        insert_version_state(
            &mut transaction,
            &namespace,
            &persisted,
            status_to_db(persisted.status),
        )
        .await?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| DomainError::Validation("skill revision overflow".into()))?;
        let result = sqlx::query(
            "UPDATE skill_heads SET current_version = ?, status = ?, pinned_version = ?, activated_at = ?, rollback_version = ?, revision = ?, updated_at = ? WHERE namespace = ? AND skill_id = ? AND revision = ?",
        )
        .bind(&persisted.manifest.version)
        .bind(status_to_db(persisted.status))
        .bind(persisted.pinned_version.as_deref())
        .bind(persisted.activated_at.map(|value| value.to_rfc3339()))
        .bind(persisted.rollback_version.as_deref())
        .bind(revision_i64(next_revision)?)
        .bind(Utc::now().to_rfc3339())
        .bind(&namespace)
        .bind(skill.manifest.id.to_string())
        .bind(expected)
        .execute(&mut *transaction)
        .await
        .map_err(|error| persistence_error("skill head update", error))?;
        if result.rows_affected() == 0 {
            return Err(concurrency_error(
                expected_revision,
                expected_revision.saturating_add(1),
            ));
        }
        transaction
            .commit()
            .await
            .map_err(|error| persistence_error("skill update commit", error))?;
        let record = SkillRecord {
            skill: persisted.clone(),
            parsed: parsed.clone(),
            revision: next_revision,
            version_id: version_id(&persisted.manifest.id, &persisted.manifest.version),
            content_hash: candidate_hash,
            parent_version: persisted.parent_version,
            compatibility: persisted.compatibility,
        };
        self.publish_version_event("update", &record, persisted.project_id);
        Ok(record)
    }

    /// Persists a new immutable draft version without moving the active head.
    /// The editor must never replace an active artifact in place.
    pub async fn create_draft(
        &self,
        skill: &Skill,
        parsed: &ParsedSkill,
        expected_revision: u64,
    ) -> Result<(SkillRecord, bool), DomainError> {
        if skill.status != SkillStatus::Draft || skill.pinned_version.is_some() {
            return Err(DomainError::InvalidStateTransition {
                from: status_to_db(skill.status).into(),
                to: "draft editor".into(),
            });
        }
        let namespace = namespace(skill.manifest.scope, skill.project_id.as_ref())?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| persistence_error("skill draft transaction", error))?;
        let Some(head) =
            head_in_transaction(&mut transaction, &namespace, &skill.manifest.id).await?
        else {
            return Err(DomainError::NotFound("skill head not found".into()));
        };
        if head.revision != expected_revision {
            return Err(concurrency_error(expected_revision, head.revision));
        }
        if head.current_version == skill.manifest.version {
            return Err(DomainError::Duplicate(
                "skill draft version already is the current head".into(),
            ));
        }

        let compatibility =
            SkillCompatibility::from_parent(Some(&head.current_version), &skill.manifest.version)
                .map_err(|error| DomainError::Validation(error.to_string()))?;
        let mut persisted = skill.clone();
        persisted.parent_version = Some(head.current_version.clone());
        persisted.compatibility = compatibility;
        validate_input(&persisted, parsed)?;
        let content_hash = compute_content_hash(&persisted, parsed)?;
        if let Some(mut existing) = version_by_content_in_transaction(
            &mut transaction,
            &namespace,
            &skill.manifest.id,
            &content_hash,
        )
        .await?
        {
            existing.revision = expected_revision;
            transaction
                .commit()
                .await
                .map_err(|error| persistence_error("skill draft dedupe commit", error))?;
            return Ok((existing, false));
        }

        insert_version(
            &mut transaction,
            &namespace,
            &persisted,
            parsed,
            persisted.parent_version.as_deref(),
            compatibility,
            &content_hash,
        )
        .await?;
        insert_version_state(
            &mut transaction,
            &namespace,
            &persisted,
            status_to_db(SkillStatus::Draft),
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|error| persistence_error("skill draft commit", error))?;
        let record = SkillRecord {
            version_id: version_id(&persisted.manifest.id, &persisted.manifest.version),
            content_hash,
            parent_version: persisted.parent_version.clone(),
            compatibility,
            skill: persisted,
            parsed: parsed.clone(),
            revision: expected_revision,
        };
        self.publish_version_event("draft", &record, record.skill.project_id);
        Ok((record, true))
    }

    /// Archives a draft version without changing the active or current head.
    pub async fn discard_draft(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        skill_id: &SkillId,
        version: &str,
        expected_revision: u64,
    ) -> Result<SkillRecord, DomainError> {
        let namespace = namespace(scope, project_id)?;
        let Some(head) = fetch_head(&self.pool, &namespace, skill_id).await? else {
            return Err(DomainError::NotFound("skill head not found".into()));
        };
        if head.revision != expected_revision {
            return Err(concurrency_error(expected_revision, head.revision));
        }
        if head.current_version == version {
            return Err(DomainError::InvalidStateTransition {
                from: "current skill version".into(),
                to: "discard draft".into(),
            });
        }
        let Some(target) = self
            .get_version(scope, project_id, skill_id, version)
            .await?
        else {
            return Err(DomainError::NotFound(
                "skill draft version not found".into(),
            ));
        };
        if !matches!(
            target.skill.status,
            SkillStatus::Draft | SkillStatus::Testing
        ) {
            return Err(DomainError::InvalidStateTransition {
                from: status_to_db(target.skill.status).into(),
                to: "archived".into(),
            });
        }
        update_version_state(&self.pool, &namespace, skill_id, version, "archived").await?;
        let mut record = self
            .get_version(scope, project_id, skill_id, version)
            .await?
            .ok_or_else(|| DomainError::NotFound("skill draft disappeared".into()))?;
        record.revision = expected_revision;
        self.publish_version_event("discard", &record, project_id.copied());
        Ok(record)
    }

    /// Promotes the current immutable version through an explicit lifecycle
    /// operation. The method never rewrites the artifact or moves the head to
    /// an unrequested version.
    pub async fn promote(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        skill_id: &SkillId,
        version: &str,
        expected_revision: u64,
    ) -> Result<SkillRecord, DomainError> {
        let namespace = namespace(scope, project_id)?;
        let Some(current) = self.get_current_by_namespace(&namespace, skill_id).await? else {
            return Err(DomainError::NotFound("skill head not found".into()));
        };
        if current.revision != expected_revision {
            return Err(concurrency_error(expected_revision, current.revision));
        }
        if current.skill.manifest.version != version {
            return Err(DomainError::NotFound(
                "skill version is not the current head".into(),
            ));
        }
        if current.compatibility == SkillCompatibility::Incompatible {
            return Err(DomainError::PermissionDenied {
                capability: "skill.activate".into(),
                reason: "incompatible skill version cannot be promoted without an explicit compatibility decision".into(),
            });
        }
        if current.skill.status == SkillStatus::Active {
            return Ok(current);
        }
        if !matches!(
            current.skill.status,
            SkillStatus::Draft | SkillStatus::Testing
        ) {
            return Err(DomainError::InvalidStateTransition {
                from: status_to_db(current.skill.status).into(),
                to: "active".into(),
            });
        }
        let mut promoted = current.skill.clone();
        if promoted.status == SkillStatus::Draft {
            promoted
                .transition(SkillStatus::Testing)
                .map_err(skill_state_error)?;
        }
        promoted
            .activate(version.to_owned())
            .map_err(skill_state_error)?;
        promoted.rollback_version = promoted.parent_version.clone();
        validate_input(&promoted, &current.parsed)?;
        let next_revision = next_revision(expected_revision)?;
        let result = sqlx::query(
            "UPDATE skill_heads SET status = 'active', pinned_version = ?, activated_at = ?, rollback_version = ?, revision = ?, updated_at = ? WHERE namespace = ? AND skill_id = ? AND revision = ?",
        )
        .bind(promoted.pinned_version.as_deref())
        .bind(promoted.activated_at.map(|value| value.to_rfc3339()))
        .bind(promoted.parent_version.as_deref())
        .bind(revision_i64(next_revision)?)
        .bind(Utc::now().to_rfc3339())
        .bind(&namespace)
        .bind(skill_id.to_string())
        .bind(revision_i64(expected_revision)?)
        .execute(&self.pool)
        .await
        .map_err(|error| persistence_error("skill promote", error))?;
        if result.rows_affected() == 0 {
            return Err(concurrency_error(expected_revision, next_revision));
        }
        update_version_state(&self.pool, &namespace, skill_id, version, "active").await?;
        let record = SkillRecord {
            skill: promoted,
            parsed: current.parsed,
            revision: next_revision,
            version_id: current.version_id,
            content_hash: current.content_hash,
            parent_version: current.parent_version,
            compatibility: current.compatibility,
        };
        self.publish_version_event("promote", &record, project_id.copied());
        Ok(record)
    }

    pub async fn archive(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        skill_id: &SkillId,
        expected_revision: u64,
    ) -> Result<SkillRecord, DomainError> {
        let namespace = namespace(scope, project_id)?;
        let Some(current) = self.get_current_by_namespace(&namespace, skill_id).await? else {
            return Err(DomainError::NotFound("skill head not found".into()));
        };
        if current.revision != expected_revision {
            return Err(concurrency_error(expected_revision, current.revision));
        }
        if current.skill.status == SkillStatus::Archived {
            return Ok(current);
        }
        let mut transitioned = current.skill.clone();
        transitioned
            .transition(SkillStatus::Archived)
            .map_err(skill_state_error)?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| DomainError::Validation("skill revision overflow".into()))?;
        let result = sqlx::query(
            "UPDATE skill_heads SET status = 'archived', revision = ?, updated_at = ? WHERE namespace = ? AND skill_id = ? AND revision = ?",
        )
        .bind(revision_i64(next_revision)?)
        .bind(Utc::now().to_rfc3339())
        .bind(&namespace)
        .bind(skill_id.to_string())
        .bind(revision_i64(expected_revision)?)
        .execute(&self.pool)
        .await
        .map_err(|error| persistence_error("skill archive", error))?;
        if result.rows_affected() == 0 {
            return Err(concurrency_error(expected_revision, next_revision));
        }
        update_version_state(
            &self.pool,
            &namespace,
            skill_id,
            &current.skill.manifest.version,
            "archived",
        )
        .await?;
        let record = self
            .get_current_by_namespace(&namespace, skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("skill head disappeared".into()))?;
        self.publish_version_event("archive", &record, project_id.copied());
        Ok(record)
    }

    pub async fn rollback(
        &self,
        scope: SkillScope,
        project_id: Option<&ProjectId>,
        skill_id: &SkillId,
        target_version: &str,
        expected_revision: u64,
    ) -> Result<SkillRecord, DomainError> {
        let namespace = namespace(scope, project_id)?;
        let Some(current) = self.get_current_by_namespace(&namespace, skill_id).await? else {
            return Err(DomainError::NotFound("skill head not found".into()));
        };
        if current.revision != expected_revision {
            return Err(concurrency_error(expected_revision, current.revision));
        }
        if !matches!(
            current.skill.status,
            SkillStatus::Active | SkillStatus::Deprecated
        ) {
            return Err(DomainError::InvalidStateTransition {
                from: status_to_db(current.skill.status).into(),
                to: "active".into(),
            });
        }
        let Some(target) = self
            .get_version(scope, project_id, skill_id, target_version)
            .await?
        else {
            return Err(DomainError::NotFound(
                "skill rollback version not found".into(),
            ));
        };
        if target.compatibility == SkillCompatibility::Incompatible {
            return Err(DomainError::PermissionDenied {
                capability: "skill.activate".into(),
                reason: "incompatible skill version cannot be restored by rollback".into(),
            });
        }
        let mut restored = target.skill.clone();
        restored.status = SkillStatus::Active;
        restored.pinned_version = Some(target_version.to_owned());
        restored.activated_at = Some(Utc::now());
        restored.rollback_version = Some(current.skill.manifest.version.clone());
        validate_input(&restored, &target.parsed)?;

        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| DomainError::Validation("skill revision overflow".into()))?;
        let result = sqlx::query(
            "UPDATE skill_heads SET current_version = ?, status = 'active', pinned_version = ?, activated_at = ?, rollback_version = ?, revision = ?, updated_at = ? WHERE namespace = ? AND skill_id = ? AND revision = ?",
        )
        .bind(target_version)
        .bind(target_version)
        .bind(restored.activated_at.map(|value| value.to_rfc3339()))
        .bind(restored.rollback_version.as_deref())
        .bind(revision_i64(next_revision)?)
        .bind(Utc::now().to_rfc3339())
        .bind(&namespace)
        .bind(skill_id.to_string())
        .bind(revision_i64(expected_revision)?)
        .execute(&self.pool)
        .await
        .map_err(|error| persistence_error("skill rollback", error))?;
        if result.rows_affected() == 0 {
            return Err(concurrency_error(expected_revision, next_revision));
        }
        update_version_state(&self.pool, &namespace, skill_id, target_version, "active").await?;
        let record = SkillRecord {
            version_id: version_id(&restored.manifest.id, &restored.manifest.version),
            content_hash: target.content_hash,
            parent_version: restored.parent_version.clone(),
            compatibility: restored.compatibility,
            skill: restored,
            parsed: target.parsed,
            revision: next_revision,
        };
        self.publish_version_event("rollback", &record, project_id.copied());
        Ok(record)
    }

    async fn get_current_by_namespace(
        &self,
        namespace: &str,
        skill_id: &SkillId,
    ) -> Result<Option<SkillRecord>, DomainError> {
        let Some(head) = fetch_head(&self.pool, namespace, skill_id).await? else {
            return Ok(None);
        };
        let row = sqlx::query(
            "SELECT sv.skill_json, sv.parsed_json, sv.content_hash, sv.parent_version, sv.compatibility, sv.hash_algorithm, states.status AS release_status FROM skill_versions AS sv LEFT JOIN skill_version_states AS states ON states.namespace = sv.namespace AND states.skill_id = sv.skill_id AND states.manifest_version = sv.manifest_version WHERE sv.namespace = ? AND sv.skill_id = ? AND sv.manifest_version = ?",
        )
        .bind(namespace)
        .bind(skill_id.to_string())
        .bind(&head.current_version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| persistence_error("skill current version query", error))?;
        row.map(|row| decode_current(&row, &head)).transpose()
    }

    fn publish_version_event(
        &self,
        action: &str,
        record: &SkillRecord,
        project_id: Option<ProjectId>,
    ) -> Option<EventId> {
        let event_id = EventId::new();
        let payload = serde_json::json!({
            "action": action,
            "version_id": record.version_id,
            "version": record.skill.manifest.version,
            "content_hash": record.content_hash,
            "parent_version": record.parent_version,
            "compatibility": record.compatibility,
            "scope": record.skill.manifest.scope,
            "source": source_event_metadata(&record.skill.manifest.source),
            "policy": record.skill.manifest.policy,
            "budget": record.skill.manifest.budget,
            "trace": record.skill.manifest.trace,
            "revision": record.revision,
        })
        .to_string();
        match project_id {
            Some(project_id) => {
                let bus = self.event_bus.as_ref()?;
                let event = ApplicationEvent {
                    schema_version: 1,
                    event_id,
                    event_type: EventKind::SkillVersionChanged,
                    project_id,
                    aggregate_id: record.skill.manifest.id.to_string(),
                    agent_id: None,
                    session_id: None,
                    occurred_at: Utc::now(),
                    sequence: record.revision,
                    payload,
                };
                let _ = bus.publish(event);
                Some(event_id)
            }
            None => {
                let bus = self.global_event_bus.as_ref()?;
                let event = GlobalApplicationEvent {
                    schema_version: 1,
                    event_id,
                    event_type: EventKind::SkillVersionChanged,
                    aggregate_id: record.skill.manifest.id.to_string(),
                    occurred_at: Utc::now(),
                    sequence: record.revision,
                    payload,
                };
                let _ = bus.publish(event);
                Some(event_id)
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Head {
    current_version: String,
    status: SkillStatus,
    pinned_version: Option<String>,
    activated_at: Option<DateTime<Utc>>,
    rollback_version: Option<String>,
    revision: u64,
}

async fn insert_version(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    namespace: &str,
    skill: &Skill,
    parsed: &ParsedSkill,
    parent_version: Option<&str>,
    compatibility: SkillCompatibility,
    content_hash: &str,
) -> Result<(), DomainError> {
    let result = sqlx::query(
        "INSERT INTO skill_versions (namespace, skill_id, project_id, scope, name, manifest_version, content_hash, skill_json, parsed_json, created_at, parent_version, compatibility, hash_algorithm) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'sha256-v1')",
    )
    .bind(namespace)
    .bind(skill.manifest.id.to_string())
    .bind(skill.project_id.map(|id| id.to_string()))
    .bind(scope_to_db(skill.manifest.scope))
    .bind(&skill.manifest.name)
    .bind(&skill.manifest.version)
    .bind(content_hash)
    .bind(serde_json::to_string(skill)?)
    .bind(serde_json::to_string(parsed)?)
    .bind(skill.manifest.created_at.to_rfc3339())
    .bind(parent_version)
    .bind(compatibility_to_db(compatibility))
    .execute(&mut **transaction)
    .await;
    match result {
        Ok(_) => Ok(()),
        Err(error) if is_unique_violation(&error) => Err(DomainError::Duplicate(
            "skill content or manifest version already exists".into(),
        )),
        Err(error) => Err(persistence_error("skill version insert", error)),
    }
}

async fn insert_version_state(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    namespace: &str,
    skill: &Skill,
    status: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO skill_version_states (namespace, skill_id, manifest_version, status, revision, updated_at) VALUES (?, ?, ?, ?, 1, ?)",
    )
    .bind(namespace)
    .bind(skill.manifest.id.to_string())
    .bind(&skill.manifest.version)
    .bind(status)
    .bind(skill.manifest.created_at.to_rfc3339())
    .execute(&mut **transaction)
    .await
    .map_err(|error| persistence_error("skill version state insert", error))?;
    Ok(())
}

async fn update_version_state(
    pool: &Pool<Sqlite>,
    namespace: &str,
    skill_id: &SkillId,
    version: &str,
    status: &str,
) -> Result<(), DomainError> {
    let result = sqlx::query(
        "UPDATE skill_version_states SET status = ?, revision = revision + 1, updated_at = ? WHERE namespace = ? AND skill_id = ? AND manifest_version = ?",
    )
    .bind(status)
    .bind(Utc::now().to_rfc3339())
    .bind(namespace)
    .bind(skill_id.to_string())
    .bind(version)
    .execute(pool)
    .await
    .map_err(|error| persistence_error("skill version state update", error))?;
    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound(
            "skill version state not found".into(),
        ));
    }
    Ok(())
}

async fn fetch_head(
    pool: &Pool<Sqlite>,
    namespace: &str,
    skill_id: &SkillId,
) -> Result<Option<Head>, DomainError> {
    let row = sqlx::query(
        "SELECT current_version, status, pinned_version, activated_at, rollback_version, revision FROM skill_heads WHERE namespace = ? AND skill_id = ?",
    )
    .bind(namespace)
    .bind(skill_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|error| persistence_error("skill head query", error))?;
    row.map(decode_head).transpose()
}

async fn version_by_content_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    namespace: &str,
    skill_id: &SkillId,
    content_hash: &str,
) -> Result<Option<SkillRecord>, DomainError> {
    let rows = sqlx::query(
        "SELECT sv.skill_json, sv.parsed_json, sv.content_hash, sv.parent_version, sv.compatibility, sv.hash_algorithm, states.status AS release_status FROM skill_versions AS sv LEFT JOIN skill_version_states AS states ON states.namespace = sv.namespace AND states.skill_id = sv.skill_id AND states.manifest_version = sv.manifest_version WHERE sv.namespace = ? AND sv.skill_id = ? ORDER BY sv.created_at ASC, sv.manifest_version ASC",
    )
    .bind(namespace)
    .bind(skill_id.to_string())
    .fetch_all(&mut **transaction)
    .await
    .map_err(|error| persistence_error("skill content deduplication query", error))?;
    for row in rows {
        let record = decode_version(&row, 0)?;
        if record.content_hash == content_hash {
            return Ok(Some(record));
        }
    }
    Ok(None)
}

async fn head_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    namespace: &str,
    skill_id: &SkillId,
) -> Result<Option<Head>, DomainError> {
    let row = sqlx::query(
        "SELECT current_version, status, pinned_version, activated_at, rollback_version, revision FROM skill_heads WHERE namespace = ? AND skill_id = ?",
    )
    .bind(namespace)
    .bind(skill_id.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| persistence_error("skill head transaction query", error))?;
    row.map(decode_head).transpose()
}

fn decode_head(row: sqlx::sqlite::SqliteRow) -> Result<Head, DomainError> {
    Ok(Head {
        current_version: row_string(&row, "current_version")?,
        status: status_from_db(&row_string(&row, "status")?)?,
        pinned_version: row_optional_string(&row, "pinned_version")?,
        activated_at: parse_optional_time(row_optional_string(&row, "activated_at")?)?,
        rollback_version: row_optional_string(&row, "rollback_version")?,
        revision: nonnegative_revision(row_i64(&row, "revision")?)?,
    })
}

fn decode_current(row: &sqlx::sqlite::SqliteRow, head: &Head) -> Result<SkillRecord, DomainError> {
    let mut record = decode_version(row, head.revision)?;
    record.skill.status = head.status;
    record.skill.pinned_version = head.pinned_version.clone();
    record.skill.activated_at = head.activated_at;
    record.skill.rollback_version = head.rollback_version.clone();
    record.skill.validate().map_err(skill_state_error)?;
    Ok(record)
}

fn decode_version(
    row: &sqlx::sqlite::SqliteRow,
    revision: u64,
) -> Result<SkillRecord, DomainError> {
    let mut skill: Skill = serde_json::from_str(&row_string(row, "skill_json")?)?;
    let parsed: ParsedSkill = serde_json::from_str(&row_string(row, "parsed_json")?)?;
    let stored_hash = row_string(row, "content_hash")?;
    let hash_algorithm = row_string(row, "hash_algorithm")?;
    let content_hash = compute_content_hash(&skill, &parsed)?;
    let parent_version = row_optional_string(row, "parent_version")?;
    let compatibility = compatibility_from_db(&row_string(row, "compatibility")?)?;
    let release_status = row_optional_string(row, "release_status")?;
    if skill.manifest.id != parsed.manifest.id
        || skill.manifest.version != parsed.manifest.version
        || skill.manifest.digest != parsed.manifest.digest
        || skill.parent_version != parent_version
        || skill.compatibility != compatibility
        || (hash_algorithm == "sha256-v1" && stored_hash != content_hash)
        || (hash_algorithm != "legacy" && hash_algorithm != "sha256-v1")
    {
        return Err(DomainError::Validation(
            "skill version payload mismatch".into(),
        ));
    }
    if let Some(status) = release_status {
        skill.status = status_from_db(&status)?;
        if skill.status == SkillStatus::Active && skill.pinned_version.is_none() {
            skill.pinned_version = Some(skill.manifest.version.clone());
            skill.activated_at = Some(skill.manifest.created_at);
        }
    }
    skill.validate().map_err(skill_state_error)?;
    Ok(SkillRecord {
        version_id: version_id(&skill.manifest.id, &skill.manifest.version),
        content_hash,
        parent_version,
        compatibility,
        skill,
        parsed,
        revision,
    })
}

fn validate_input(skill: &Skill, parsed: &ParsedSkill) -> Result<(), DomainError> {
    skill.validate().map_err(skill_state_error)?;
    if skill.manifest.id != parsed.manifest.id
        || skill.manifest.version != parsed.manifest.version
        || skill.manifest.digest != parsed.manifest.digest
        || skill.manifest.scope != parsed.manifest.scope
    {
        return Err(DomainError::Validation(
            "skill payload does not match manifest".into(),
        ));
    }
    if parsed.quarantined && skill.status == SkillStatus::Active {
        return Err(DomainError::PermissionDenied {
            capability: "skill.activate".into(),
            reason: "quarantined skill cannot become active".into(),
        });
    }
    if skill.status == SkillStatus::Active && skill.manifest.policy.requires_approval {
        return Err(DomainError::PermissionDenied {
            capability: "skill.activate".into(),
            reason: "explicit approval is required before activation".into(),
        });
    }
    for section in &parsed.instructions {
        reject_sensitive(section.content.as_str())?;
    }
    for artifact in &parsed.artifacts {
        reject_sensitive(&artifact.content)?;
    }
    Ok(())
}

fn reject_sensitive(value: &str) -> Result<(), DomainError> {
    let lower = value.to_ascii_lowercase();
    if [
        "authorization:",
        "bearer ",
        "api_key=",
        "api-key=",
        "password=",
        "private key",
        "secret=",
        "access_token=",
        "client_secret=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return Err(DomainError::Validation(
            "skill payload contains sensitive material".into(),
        ));
    }
    Ok(())
}

fn namespace(scope: SkillScope, project_id: Option<&ProjectId>) -> Result<String, DomainError> {
    match (scope, project_id) {
        (SkillScope::Project, Some(project_id)) => Ok(format!("project:{project_id}")),
        (SkillScope::Global, None) => Ok("global".into()),
        (SkillScope::Project, None) => Err(DomainError::Validation(
            "project skill query requires project identity".into(),
        )),
        (SkillScope::Global, Some(_)) => Err(DomainError::Validation(
            "global skill query cannot carry project identity".into(),
        )),
    }
}

fn scope_to_db(scope: SkillScope) -> &'static str {
    match scope {
        SkillScope::Project => "project",
        SkillScope::Global => "global",
    }
}

fn status_to_db(status: SkillStatus) -> &'static str {
    match status {
        SkillStatus::Draft => "draft",
        SkillStatus::Testing => "testing",
        SkillStatus::Active => "active",
        SkillStatus::Deprecated => "deprecated",
        SkillStatus::Archived => "archived",
        SkillStatus::Blocked => "blocked",
    }
}

fn compatibility_to_db(value: SkillCompatibility) -> &'static str {
    match value {
        SkillCompatibility::Initial => "initial",
        SkillCompatibility::Compatible => "compatible",
        SkillCompatibility::Incompatible => "incompatible",
    }
}

fn compatibility_from_db(value: &str) -> Result<SkillCompatibility, DomainError> {
    match value {
        "initial" => Ok(SkillCompatibility::Initial),
        "compatible" => Ok(SkillCompatibility::Compatible),
        "incompatible" => Ok(SkillCompatibility::Incompatible),
        _ => Err(DomainError::Validation(
            "invalid skill compatibility".into(),
        )),
    }
}

fn version_id(skill_id: &SkillId, version: &str) -> String {
    format!("{skill_id}@{version}")
}

fn compute_content_hash(_skill: &Skill, parsed: &ParsedSkill) -> Result<String, DomainError> {
    // Version labels, declared digests, creation timestamps and parser trace
    // data are provenance envelopes rather than content. The remaining
    // validated manifest policy plus parsed instructions/artifacts form the
    // stable identity used for safe retry deduplication.
    let manifest = &parsed.manifest;
    let canonical = serde_json::json!({
        "manifest": {
            "schema_version": manifest.schema_version,
            "id": manifest.id,
            "name": manifest.name,
            "description": manifest.description,
            "author": manifest.author,
            "license": manifest.license,
            "repository": manifest.repository,
            "source": manifest.source,
            "scope": manifest.scope,
            "capabilities": manifest.capabilities,
            "dependencies": manifest.dependencies,
            "files": manifest.files,
            "tests": manifest.tests,
            "policy": manifest.policy,
            "budget": manifest.budget,
        },
        "instructions": parsed.instructions,
        "artifacts": parsed.artifacts,
        "links": parsed.links,
        "diagnostics": parsed.diagnostics,
        "quarantined": parsed.quarantined,
    });
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn source_event_metadata(source: &agent_core::SkillSource) -> serde_json::Value {
    serde_json::json!({
        "kind": source.kind,
        "reference_digest": format!("{:x}", Sha256::digest(source.reference.as_bytes())),
    })
}

fn next_revision(current: u64) -> Result<u64, DomainError> {
    current
        .checked_add(1)
        .ok_or_else(|| DomainError::Validation("skill revision overflow".into()))
}

fn status_from_db(value: &str) -> Result<SkillStatus, DomainError> {
    match value {
        "draft" => Ok(SkillStatus::Draft),
        "testing" => Ok(SkillStatus::Testing),
        "active" => Ok(SkillStatus::Active),
        "deprecated" => Ok(SkillStatus::Deprecated),
        "archived" => Ok(SkillStatus::Archived),
        "blocked" => Ok(SkillStatus::Blocked),
        _ => Err(DomainError::Validation("invalid skill status".into())),
    }
}

fn parse_skill_id(value: &str) -> Result<SkillId, DomainError> {
    SkillId::from_str(value).map_err(|_| DomainError::Validation("invalid skill id".into()))
}

fn row_string(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<String, DomainError> {
    row.try_get(column)
        .map_err(|_| DomainError::InvariantViolation("skill row decode failed".into()))
}

fn row_optional_string(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
) -> Result<Option<String>, DomainError> {
    row.try_get(column)
        .map_err(|_| DomainError::InvariantViolation("skill row decode failed".into()))
}

fn row_i64(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<i64, DomainError> {
    row.try_get(column)
        .map_err(|_| DomainError::InvariantViolation("skill row decode failed".into()))
}

fn parse_optional_time(value: Option<String>) -> Result<Option<DateTime<Utc>>, DomainError> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|_| DomainError::Validation("invalid skill timestamp".into()))
        })
        .transpose()
}

fn nonnegative_revision(value: i64) -> Result<u64, DomainError> {
    u64::try_from(value).map_err(|_| DomainError::Validation("invalid skill revision".into()))
}

fn revision_i64(value: u64) -> Result<i64, DomainError> {
    i64::try_from(value).map_err(|_| DomainError::Validation("skill revision overflow".into()))
}

fn concurrency_error(expected: u64, actual: u64) -> DomainError {
    DomainError::ConcurrencyConflict {
        expected: expected.to_string(),
        actual: actual.to_string(),
    }
}

fn skill_state_error(error: SkillError) -> DomainError {
    DomainError::Validation(error.to_string())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation())
}

fn persistence_error(context: &str, _error: impl std::fmt::Display) -> DomainError {
    DomainError::InvariantViolation(format!("{context} failed"))
}
