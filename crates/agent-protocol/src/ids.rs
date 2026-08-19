//! IDs tipados para todas as entidades do domínio.
//!
//! Tipos fortes que impedem confusão entre diferentes tipos de identidade
//! e carregam validação de formato no nível de tipo.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Macro para criar IDs tipados com validação
macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}-{}", $prefix, self.0)
            }
        }

        impl FromStr for $name {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let s = s
                    .strip_prefix(concat!($prefix, "-"))
                    .ok_or_else(|| anyhow::anyhow!("invalid {} id prefix", $prefix))?;
                Ok(Self(Uuid::parse_str(s)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

typed_id!(ProjectId, "proj");
typed_id!(AgentId, "agent");
typed_id!(SessionId, "sess");
typed_id!(MessageId, "msg");
typed_id!(WorkflowId, "wf");
typed_id!(NodeId, "node");
typed_id!(RunId, "run");
typed_id!(TraceId, "trace");
typed_id!(RequestId, "req");
typed_id!(SkillId, "skill");
typed_id!(MemoryId, "mem");
typed_id!(ToolId, "tool");
typed_id!(ProviderId, "prov");
typed_id!(CredentialId, "cred");
typed_id!(GroupId, "grp");
typed_id!(TaskId, "task");
typed_id!(ArtifactId, "art");

#[cfg(test)]
mod tests {
    use super::{AgentId, ProjectId};
    use std::str::FromStr;

    #[test]
    fn typed_ids_roundtrip_with_required_prefix() {
        let id = ProjectId::new();
        let text = id.to_string();
        assert!(text.starts_with("proj-"));
        assert_eq!(ProjectId::from_str(&text).unwrap(), id);
    }

    #[test]
    fn typed_ids_reject_wrong_prefix_and_malformed_values() {
        let project = ProjectId::new();
        assert!(AgentId::from_str(&project.to_string()).is_err());
        assert!(ProjectId::from_str("proj-not-a-uuid").is_err());
        assert!(ProjectId::from_str(&project.as_uuid().to_string()).is_err());
    }
}
