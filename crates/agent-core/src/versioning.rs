//! Immutable Skill version metadata and compatibility rules.

use semver::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCompatibility {
    /// The first version in a Skill history has no parent to compare.
    #[default]
    Initial,
    /// The candidate keeps the parent major version and is eligible for
    /// explicit promotion after validation/approval.
    Compatible,
    /// A major-version change requires a separate compatibility decision and
    /// cannot be activated by the ordinary version update path.
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SkillVersionError {
    #[error("skill version is invalid")]
    InvalidVersion,
    #[error("skill version parent cannot equal the candidate version")]
    SelfParent,
}

impl SkillCompatibility {
    pub fn from_parent(parent: Option<&str>, candidate: &str) -> Result<Self, SkillVersionError> {
        let candidate = Version::parse(candidate).map_err(|_| SkillVersionError::InvalidVersion)?;
        let Some(parent) = parent else {
            return Ok(Self::Initial);
        };
        if parent == candidate.to_string() {
            return Err(SkillVersionError::SelfParent);
        }
        let parent = Version::parse(parent).map_err(|_| SkillVersionError::InvalidVersion)?;
        Ok(if parent.major == candidate.major {
            Self::Compatible
        } else {
            Self::Incompatible
        })
    }
}
