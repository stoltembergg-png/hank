//! SQLite repository for immutable Skill versions and scoped mutable heads.
//!
//! The repository stores parsed content as untrusted data only. It never
//! resolves references, imports global Skills implicitly, changes runtime
//! state, or executes an artifact.

use agent_core::{
    DomainError, ParsedSkill, ProjectId, Skill, SkillError, SkillId, SkillScope, SkillStatus,
};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct SkillRecord {
    pub skill: Skill,
    pub parsed: ParsedSkill,
    pub revision: u64,
}

#[derive(Clone)]
pub struct SqliteSkillRepository {
    pool: Pool<Sqlite>,
}

impl SqliteSkillRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        skill: &Skill,
        parsed: &ParsedSkill,
    ) -> Result<SkillRecord, DomainError> {
        validate_input(skill, parsed)?;
        let namespace = namespace(skill.manifest.scope, skill.project_id.as_ref())?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| persistence_error("skill create transaction", error))?;

        insert_version(&mut transaction, &namespace, skill, parsed).await?;
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
        Ok(SkillRecord {
            skill: skill.clone(),
            parsed: parsed.clone(),
            revision: 1,
        })
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
            "SELECT skill_json, parsed_json FROM skill_versions WHERE namespace = ? AND skill_id = ? AND manifest_version = ?",
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
            "SELECT skill_json, parsed_json FROM skill_versions WHERE namespace = ? AND skill_id = ? ORDER BY created_at ASC, manifest_version ASC",
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
        validate_input(skill, parsed)?;
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
        if head.current_version == skill.manifest.version {
            return Err(DomainError::Duplicate(
                "skill manifest version is immutable".into(),
            ));
        }
        insert_version(&mut transaction, &namespace, skill, parsed).await?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| DomainError::Validation("skill revision overflow".into()))?;
        let result = sqlx::query(
            "UPDATE skill_heads SET current_version = ?, status = ?, pinned_version = ?, activated_at = ?, rollback_version = ?, revision = ?, updated_at = ? WHERE namespace = ? AND skill_id = ? AND revision = ?",
        )
        .bind(&skill.manifest.version)
        .bind(status_to_db(skill.status))
        .bind(skill.pinned_version.as_deref())
        .bind(skill.activated_at.map(|value| value.to_rfc3339()))
        .bind(skill.rollback_version.as_deref())
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
        Ok(SkillRecord {
            skill: skill.clone(),
            parsed: parsed.clone(),
            revision: next_revision,
        })
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
        self.get_current_by_namespace(&namespace, skill_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("skill head disappeared".into()))
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
        Ok(SkillRecord {
            skill: restored,
            parsed: target.parsed,
            revision: next_revision,
        })
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
            "SELECT skill_json, parsed_json FROM skill_versions WHERE namespace = ? AND skill_id = ? AND manifest_version = ?",
        )
        .bind(namespace)
        .bind(skill_id.to_string())
        .bind(&head.current_version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| persistence_error("skill current version query", error))?;
        row.map(|row| decode_current(&row, &head)).transpose()
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
) -> Result<(), DomainError> {
    let result = sqlx::query(
        "INSERT INTO skill_versions (namespace, skill_id, project_id, scope, name, manifest_version, content_hash, skill_json, parsed_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(namespace)
    .bind(skill.manifest.id.to_string())
    .bind(skill.project_id.map(|id| id.to_string()))
    .bind(scope_to_db(skill.manifest.scope))
    .bind(&skill.manifest.name)
    .bind(&skill.manifest.version)
    .bind(&skill.manifest.digest)
    .bind(serde_json::to_string(skill)?)
    .bind(serde_json::to_string(parsed)?)
    .bind(skill.manifest.created_at.to_rfc3339())
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
    let skill: Skill = serde_json::from_str(&row_string(row, "skill_json")?)?;
    let parsed: ParsedSkill = serde_json::from_str(&row_string(row, "parsed_json")?)?;
    if skill.manifest.id != parsed.manifest.id
        || skill.manifest.version != parsed.manifest.version
        || skill.manifest.digest != parsed.manifest.digest
    {
        return Err(DomainError::Validation(
            "skill version payload mismatch".into(),
        ));
    }
    skill.validate().map_err(skill_state_error)?;
    Ok(SkillRecord {
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
