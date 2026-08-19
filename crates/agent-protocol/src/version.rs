//! Versionamento de protocolo e compatibilidade.
//!
//! Define regras de compatibilidade entre versões de protocolo,
//! negociação de versão e migração de schemas.

use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Versão do protocolo com metadados de compatibilidade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub version: Version,
    pub min_compatible: Version,
    pub deprecated_since: Option<Version>,
    pub removed_since: Option<Version>,
    pub migration_path: Option<String>,
}

impl ProtocolVersion {
    pub fn current() -> Self {
        Self {
            version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            min_compatible: Version::parse("0.1.0").unwrap(),
            deprecated_since: None,
            removed_since: None,
            migration_path: None,
        }
    }

    pub fn is_compatible(&self, other: &Version) -> bool {
        other >= &self.min_compatible && other <= &self.version
    }
}

/// Registro de versões de schemas conhecidos
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaRegistry {
    pub schemas: HashMap<String, ProtocolVersion>,
}

impl SchemaRegistry {
    pub fn register(&mut self, name: String, version: ProtocolVersion) {
        self.schemas.insert(name, version);
    }

    pub fn get(&self, name: &str) -> Option<&ProtocolVersion> {
        self.schemas.get(name)
    }

    pub fn is_compatible(&self, name: &str, version: &Version) -> bool {
        self.schemas
            .get(name)
            .map(|v| v.is_compatible(version))
            .unwrap_or(false)
    }
}

/// Negociação de versão entre duas partes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionNegotiation {
    pub client_version: Version,
    pub server_version: Version,
    pub agreed_version: Version,
    pub compatible: bool,
}

impl VersionNegotiation {
    pub fn negotiate(client: Version, server: Version) -> Self {
        let agreed = std::cmp::min(client.clone(), server.clone());
        let compatible = agreed >= Version::parse("0.1.0").unwrap();
        Self {
            client_version: client,
            server_version: server,
            agreed_version: agreed,
            compatible,
        }
    }
}
