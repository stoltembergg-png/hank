//! IDs tipados para todas as entidades do domínio.
//!
//! Tipos fortes que impedem confusão entre diferentes tipos de identidade
//! e carregam validação de formato no nível de tipo.
//!
//! Forma textual canônica: `{prefixo}-{uuid}` (ex.: `proj-<uuid>`).
//! A serialização (serde) usa sempre a forma canônica com prefixo, de modo
//! que o roundtrip preserva a identidade e o tipo. Strings sem o prefixo
//! correto, acima do limite de tamanho ou contendo caracteres de controle
//! ou travessia de caminho são rejeitadas na conversão.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Tamanho máximo (em bytes) da forma textual canônica de um ID.
///
/// O formato é `{prefixo}-{uuid}`: os prefixos atuais têm no máximo 5 chars
/// (`cred`, `skill`, `trace`, `agent`), mantendo o total bem abaixo de 64
/// bytes mesmo para prefixos futuros.
pub const MAX_ID_LEN: usize = 64;

/// Código estável de erro de validação de ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdErrorKind {
    /// Prefixo ausente ou divergente do tipo esperado.
    InvalidPrefix,
    /// Formato textual acima de [`MAX_ID_LEN`].
    TooLong,
    /// O sufixo UUID é inválido.
    InvalidFormat,
    /// Caractere de controle presente na entrada.
    ControlCharacter,
    /// Entrada contém travessia de caminho (`/`, `\` ou `..`).
    PathTraversal,
    /// Erro inesperado.
    Unexpected,
}

/// Erro de validação de um ID tipado.
///
/// Nunca carrega o payload completo: expõe apenas o tipo de ID, o código de
/// erro e um id de correlação (derivado do input) para rastreio seguro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdParseError {
    /// Tipo do ID que falhou (ex.: `"ProjectId"`).
    pub id_type: String,
    /// Código estável do erro.
    pub kind: IdErrorKind,
    /// Id de correlação para rastreio; derivado do input, não o contém.
    pub correlation_id: String,
}

impl IdParseError {
    fn new(id_type: &str, kind: IdErrorKind, input: &str) -> Self {
        Self {
            id_type: id_type.to_string(),
            kind,
            correlation_id: correlation_id(input),
        }
    }
}

impl fmt::Display for IdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "id inválido: tipo={} kind={:?} correlation_id={}",
            self.id_type, self.kind, self.correlation_id
        )
    }
}

impl std::error::Error for IdParseError {}

/// Deriva um id de correlação determinístico e redigido a partir do input.
fn correlation_id(input: &str) -> String {
    format!("{:016x}", fnv1a64(input.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Macro para criar IDs tipados com validação estrita.
macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            /// Prefixo canônico deste tipo de ID.
            pub const PREFIX: &'static str = $prefix;

            /// Gera um novo ID aleatório (UUID v4).
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Constrói um ID a partir de um UUID já válido.
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Expõe o UUID interno.
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Valida e converte a forma textual canônica `{prefixo}-{uuid}`.
            ///
            /// Rejeita: prefixo ausente/divergente, tamanho acima de
            /// [`MAX_ID_LEN`], caracteres de controle e travessia de caminho.
            pub fn parse(input: &str) -> Result<Self, IdParseError> {
                let id_type = stringify!($name);

                if input.len() > MAX_ID_LEN {
                    return Err(IdParseError::new(id_type, IdErrorKind::TooLong, input));
                }
                if input.chars().any(char::is_control) {
                    return Err(IdParseError::new(
                        id_type,
                        IdErrorKind::ControlCharacter,
                        input,
                    ));
                }
                if input.contains('/') || input.contains('\\') || input.contains("..") {
                    return Err(IdParseError::new(
                        id_type,
                        IdErrorKind::PathTraversal,
                        input,
                    ));
                }

                let suffix = input
                    .strip_prefix(concat!($prefix, "-"))
                    .ok_or_else(|| IdParseError::new(id_type, IdErrorKind::InvalidPrefix, input))?;

                let uuid = Uuid::parse_str(suffix)
                    .map_err(|_| IdParseError::new(id_type, IdErrorKind::InvalidFormat, input))?;

                Ok(Self(uuid))
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
            type Err = IdParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdParseError;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::parse(s)
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

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let text = String::deserialize(deserializer)?;
                Self::parse(&text).map_err(serde::de::Error::custom)
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
typed_id!(EventId, "evt");

typed_id!(OperationKey, "op");

/// Garante que tipos de ID distintos não são intercambiáveis.
///
/// A identidade é transportada no tipo: misturar `ProjectId` e `AgentId`
/// em APIs que esperam um tipo específico não compila.
///
/// ```compile_fail
/// use agent_protocol::ids::{AgentId, ProjectId};
///
/// fn takes_project(_: ProjectId) {}
///
/// fn main() {
///     let agent = AgentId::new();
///     takes_project(agent);
/// }
/// ```
pub fn ids_are_not_interchangeable() {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_uuid() -> Uuid {
        Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap()
    }

    #[test]
    fn roundtrip_preserves_identity() {
        let id = ProjectId::from_uuid(sample_uuid());
        let text = id.to_string();
        assert_eq!(text, "proj-00000000-0000-4000-8000-000000000001");
        assert_eq!(text.parse::<ProjectId>().unwrap(), id);
        assert_eq!(ProjectId::try_from(text.as_str()).unwrap(), id);
    }

    #[test]
    fn serde_roundtrip_uses_prefixed_form() {
        let id = AgentId::from_uuid(sample_uuid());
        let encoded = serde_json::to_string(&id).unwrap();
        assert_eq!(encoded, "\"agent-00000000-0000-4000-8000-000000000001\"");
        let decoded: AgentId = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, id);
    }

    #[test]
    fn serde_rejects_invalid_input() {
        let result: Result<ProjectId, _> =
            serde_json::from_value(json!("00000000-0000-4000-8000-000000000001"));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_prefix() {
        let err = "00000000-0000-4000-8000-000000000001"
            .parse::<ProjectId>()
            .unwrap_err();
        assert_eq!(err.kind, IdErrorKind::InvalidPrefix);
        assert_eq!(err.id_type, "ProjectId");
        assert!(!err.correlation_id.is_empty());
    }

    #[test]
    fn rejects_wrong_prefix() {
        let err = "agent-00000000-0000-4000-8000-000000000001"
            .parse::<ProjectId>()
            .unwrap_err();
        assert_eq!(err.kind, IdErrorKind::InvalidPrefix);
    }

    #[test]
    fn rejects_invalid_uuid_format() {
        let err = "proj-not-a-uuid".parse::<ProjectId>().unwrap_err();
        assert_eq!(err.kind, IdErrorKind::InvalidFormat);
    }

    #[test]
    fn rejects_oversized_input() {
        let long = format!("proj-{}", "0".repeat(MAX_ID_LEN + 1));
        let err = long.parse::<ProjectId>().unwrap_err();
        assert_eq!(err.kind, IdErrorKind::TooLong);
    }

    #[test]
    fn rejects_control_characters() {
        let err = "proj-00000000-0000-4000-8000-000000000001\n"
            .parse::<ProjectId>()
            .unwrap_err();
        assert_eq!(err.kind, IdErrorKind::ControlCharacter);
    }

    #[test]
    fn rejects_path_traversal() {
        let cases = [
            "proj-../../etc/passwd",
            "proj-..\\..",
            "/proj-00000000-0000-4000-8000-000000000001",
        ];
        for case in cases {
            assert_eq!(
                case.parse::<ProjectId>().unwrap_err().kind,
                IdErrorKind::PathTraversal,
                "case: {case}"
            );
        }
    }

    #[test]
    fn explicit_conversions_preserve_identity() {
        let uuid = sample_uuid();
        let id = ProjectId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
        let roundtrip: Uuid = id.into();
        assert_eq!(roundtrip, uuid);
        assert_eq!(ProjectId::from(uuid), id);
    }

    #[test]
    fn error_display_is_redacted() {
        let err = "proj-SECRET-VALUE".parse::<ProjectId>().unwrap_err();
        let message = err.to_string();
        assert!(!message.contains("SECRET-VALUE"));
        assert!(message.contains("ProjectId"));
    }
}
