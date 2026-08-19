//! Entidades Skill e lifecycle de domínio.

use crate::ids::{ProjectId, SkillId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status da skill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Draft,
    Testing,
    Active,
    Deprecated,
    Archived,
    Blocked,
}

/// Manifesto da skill (versão imutável)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub id: SkillId,
    pub version: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub repository: Option<String>,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<SkillDependency>,
    pub files: HashMap<String, SkillFile>,
    pub tests: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub digest: String, // SHA256 do conteúdo
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub skill_id: SkillId,
    pub version_req: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFile {
    pub path: String,
    pub role: SkillFileRole,
    pub digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillFileRole {
    Instruction,
    Script,
    Template,
    Reference,
    Test,
    Manifest,
}

/// Skill de domínio (versão pinada por run)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub manifest: SkillManifest,
    pub status: SkillStatus,
    pub project_id: Option<ProjectId>,  // None = global
    pub pinned_version: Option<String>, // Para pin por run
    pub activated_at: Option<DateTime<Utc>>,
    pub rollback_version: Option<String>,
}

impl Skill {
    pub fn new(manifest: SkillManifest, project_id: Option<ProjectId>) -> Self {
        Self {
            manifest,
            status: SkillStatus::Draft,
            project_id,
            pinned_version: None,
            activated_at: None,
            rollback_version: None,
        }
    }

    pub fn activate(&mut self, version: String) {
        self.status = SkillStatus::Active;
        self.pinned_version = Some(version);
        self.activated_at = Some(Utc::now());
    }

    pub fn rollback(&mut self, version: String) {
        self.rollback_version = Some(self.pinned_version.clone().unwrap_or_default());
        self.pinned_version = Some(version);
        self.activated_at = Some(Utc::now());
    }

    pub fn deprecate(&mut self) {
        self.status = SkillStatus::Deprecated;
    }
}
