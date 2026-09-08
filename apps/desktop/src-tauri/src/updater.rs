//! Fail-closed signed update staging, atomic activation, and rollback.
//!
//! Network discovery and unattended download are intentionally outside this
//! module. Callers provide bytes plus a signed attestation; this layer verifies
//! the immutable policy before touching the application slots.

use base64::Engine;
use ring::signature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

const ATTESTATION_SCHEMA_V1: u32 = 1;
const ATTESTATION_SCHEMA_V2: u32 = 2;
const UPDATE_SCHEMA: u32 = 1;

#[derive(Debug, Clone)]
pub struct UpdatePolicy {
    pub channel: String,
    pub os: String,
    pub arch: String,
    pub current_version: u64,
    pub minimum_version: u64,
    pub max_bytes: u64,
    pub now: u64,
    pub repository: String,
    pub event: String,
    pub workflow: String,
    pub policy: String,
    pub trusted_key_id: String,
    /// DER-encoded SubjectPublicKeyInfo, base64 encoded like release signing.
    pub trusted_public_key_der_b64: String,
}

#[derive(Debug, Clone)]
pub struct UpdateMetadata {
    pub schema_version: u32,
    pub version: u64,
    pub channel: String,
    pub os: String,
    pub arch: String,
    pub size: u64,
    pub expires_at: u64,
    pub bytes: Vec<u8>,
    pub attestation: UpdateAttestation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAttestation {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub artifact: ArtifactIdentity,
    pub identity: BuildIdentity,
    pub signer: SignerIdentity,
    #[serde(default)]
    pub update: Option<UpdateBinding>,
    pub signature: SignatureEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactIdentity {
    pub name: String,
    pub digest: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildIdentity {
    pub repository: String,
    pub event: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub commit: String,
    pub tree: String,
    pub workflow: String,
    pub policy: String,
    pub channel: String,
    pub os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerIdentity {
    #[serde(rename = "keyId")]
    pub key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBinding {
    pub version: u64,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureEnvelope {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageMarker {
    pub schema_version: u32,
    pub version: u64,
    pub channel: String,
    pub digest: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageResult {
    pub version: u64,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryResult {
    Clean,
    RestoredPrevious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum UpdateError {
    #[error("update consent is required")]
    ConsentRequired,
    #[error("update metadata policy mismatch")]
    PolicyMismatch,
    #[error("update version is not an upgrade")]
    Downgrade,
    #[error("update metadata is expired")]
    Expired,
    #[error("update exceeds bounded size")]
    SizeLimit,
    #[error("update artifact digest mismatch")]
    DigestMismatch,
    #[error("update signature is invalid")]
    InvalidSignature,
    #[error("update signer is not trusted")]
    UntrustedSigner,
    #[error("update attestation is malformed")]
    MalformedAttestation,
    #[error("update staging is incomplete")]
    IncompleteStaging,
    #[error("update activation is unavailable")]
    ActivationUnavailable,
    #[error("update filesystem operation failed")]
    Filesystem,
}

pub struct UpdateManager {
    root: PathBuf,
    policy: UpdatePolicy,
}

impl UpdateManager {
    pub fn new(root: impl Into<PathBuf>, policy: UpdatePolicy) -> Self {
        Self { root: root.into(), policy }
    }

    pub fn validate(&self, metadata: &UpdateMetadata) -> Result<(), UpdateError> {
        if metadata.schema_version != UPDATE_SCHEMA
            || metadata.channel != self.policy.channel
            || metadata.os != self.policy.os
            || metadata.arch != self.policy.arch
        {
            return Err(UpdateError::PolicyMismatch);
        }
        if metadata.version <= self.policy.current_version || metadata.version < self.policy.minimum_version {
            return Err(UpdateError::Downgrade);
        }
        if metadata.expires_at <= self.policy.now {
            return Err(UpdateError::Expired);
        }
        if metadata.size != metadata.bytes.len() as u64 || metadata.size > self.policy.max_bytes {
            return Err(UpdateError::SizeLimit);
        }
        let digest = digest(&metadata.bytes);
        if metadata.attestation.artifact.digest != digest
            || metadata.attestation.artifact.size != metadata.size
        {
            return Err(UpdateError::DigestMismatch);
        }
        let attestation = &metadata.attestation;
        if !matches!(attestation.schema_version, ATTESTATION_SCHEMA_V1 | ATTESTATION_SCHEMA_V2)
            || attestation.artifact.name.trim().is_empty()
            || attestation.identity.repository != self.policy.repository
            || attestation.identity.event != self.policy.event
            || attestation.identity.workflow != self.policy.workflow
            || attestation.identity.policy != self.policy.policy
            || attestation.identity.channel != metadata.channel
            || attestation.identity.os != format!("{}-{}", metadata.os, metadata.arch)
            || attestation.signer.key_id != self.policy.trusted_key_id
            || attestation.signature.algorithm != "ed25519"
            || attestation.signature.value.len() > 512
        {
            return Err(UpdateError::PolicyMismatch);
        }
        if attestation.schema_version >= ATTESTATION_SCHEMA_V2
            && attestation.update.as_ref().is_none_or(|binding| {
                binding.version != metadata.version
                    || binding.expires_at != metadata.expires_at
                    || binding.os != metadata.os
                    || binding.arch != metadata.arch
            })
        {
            return Err(UpdateError::PolicyMismatch);
        }
        let public_key = base64::engine::general_purpose::STANDARD
            .decode(&self.policy.trusted_public_key_der_b64)
            .map_err(|_| UpdateError::UntrustedSigner)?;
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(&attestation.signature.value)
            .map_err(|_| UpdateError::InvalidSignature)?;
        signature::UnparsedPublicKey::new(&signature::ED25519, public_key)
            .verify(&canonical_payload(attestation)?, &signature_bytes)
            .map_err(|_| UpdateError::InvalidSignature)
    }

    pub fn stage(&self, metadata: &UpdateMetadata, consent: bool) -> Result<StageResult, UpdateError> {
        if !consent {
            return Err(UpdateError::ConsentRequired);
        }
        self.validate(metadata)?;
        let staging = self.root.join("staging");
        remove_dir_if_exists(&staging)?;
        fs::create_dir_all(&staging).map_err(|_| UpdateError::Filesystem)?;
        let result = (|| {
            let artifact = staging.join("artifact.bin");
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&artifact)
                .map_err(|_| UpdateError::Filesystem)?;
            file.write_all(&metadata.bytes).map_err(|_| UpdateError::Filesystem)?;
            file.sync_all().map_err(|_| UpdateError::Filesystem)?;
            let marker = StageMarker {
                schema_version: UPDATE_SCHEMA,
                version: metadata.version,
                channel: metadata.channel.clone(),
                digest: metadata.attestation.artifact.digest.clone(),
                size: metadata.size,
            };
            write_json_sync(&staging.join("marker.json"), &marker)?;
            Ok(StageResult { version: metadata.version, digest: marker.digest })
        })();
        if result.is_err() {
            remove_dir_if_exists(&staging)?;
        }
        result
    }

    pub fn recover_interrupted_activation(&self) -> Result<RecoveryResult, UpdateError> {
        let pending = self.root.join("activation.pending");
        if !pending.exists() {
            return Ok(RecoveryResult::Clean);
        }
        let current = self.root.join("current");
        let previous = self.root.join("previous");
        if !current.exists() && previous.exists() {
            fs::rename(&previous, &current).map_err(|_| UpdateError::ActivationUnavailable)?;
        }
        remove_file_if_exists(&pending)?;
        Ok(RecoveryResult::RestoredPrevious)
    }

    pub fn activate(&self) -> Result<(), UpdateError> {
        self.recover_interrupted_activation()?;
        let staging = self.root.join("staging");
        let current = self.root.join("current");
        let previous = self.root.join("previous");
        let artifact = staging.join("artifact.bin");
        let marker_path = staging.join("marker.json");
        if !artifact.exists() || !marker_path.exists() {
            return Err(UpdateError::IncompleteStaging);
        }
        let marker: StageMarker = serde_json::from_slice(
            &fs::read(&marker_path).map_err(|_| UpdateError::IncompleteStaging)?,
        )
        .map_err(|_| UpdateError::IncompleteStaging)?;
        let artifact_bytes = fs::read(&artifact).map_err(|_| UpdateError::IncompleteStaging)?;
        if digest(&artifact_bytes) != marker.digest {
            return Err(UpdateError::DigestMismatch);
        }
        let artifact_size = artifact_bytes.len() as u64;
        if artifact_size != marker.size || artifact_size > self.policy.max_bytes {
            return Err(UpdateError::SizeLimit);
        }
        fs::create_dir_all(&self.root).map_err(|_| UpdateError::Filesystem)?;
        write_json_sync(&self.root.join("activation.pending"), &StageMarker {
            schema_version: UPDATE_SCHEMA,
            version: 0,
            channel: self.policy.channel.clone(),
            digest: "pending".into(),
            size: 0,
        })?;
        if previous.exists() {
            remove_dir_if_exists(&previous)?;
        }
        if current.exists() {
            fs::rename(&current, &previous).map_err(|_| UpdateError::ActivationUnavailable)?;
        }
        fs::rename(&staging, &current).map_err(|_| UpdateError::ActivationUnavailable)?;
        remove_file_if_exists(&self.root.join("activation.pending"))?;
        Ok(())
    }

    pub fn rollback(&self) -> Result<(), UpdateError> {
        let current = self.root.join("current");
        let previous = self.root.join("previous");
        if !current.exists() || !previous.exists() {
            return Err(UpdateError::ActivationUnavailable);
        }
        let swap = self.root.join("rollback.swap");
        remove_dir_if_exists(&swap)?;
        fs::rename(&current, &swap).map_err(|_| UpdateError::ActivationUnavailable)?;
        if let Err(error) = fs::rename(&previous, &current) {
            let _ = fs::rename(&swap, &current);
            return Err(if error.kind() == std::io::ErrorKind::NotFound {
                UpdateError::ActivationUnavailable
            } else {
                UpdateError::Filesystem
            });
        }
        fs::rename(&swap, &previous).map_err(|_| UpdateError::ActivationUnavailable)?;
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn canonical_payload(attestation: &UpdateAttestation) -> Result<Vec<u8>, UpdateError> {
    #[derive(Serialize)]
    struct Canonical<'a> {
        #[serde(rename = "schemaVersion")]
        schema_version: u32,
        artifact: &'a ArtifactIdentity,
        identity: CanonicalIdentity<'a>,
        signer: CanonicalSigner<'a>,
        #[serde(skip_serializing_if = "Option::is_none")]
        update: Option<&'a UpdateBinding>,
    }
    #[derive(Serialize)]
    struct CanonicalIdentity<'a> {
        repository: &'a str,
        event: &'a str,
        #[serde(rename = "ref")]
        ref_name: &'a str,
        commit: &'a str,
        tree: &'a str,
        workflow: &'a str,
        policy: &'a str,
        channel: &'a str,
        os: &'a str,
    }
    #[derive(Serialize)]
    struct CanonicalSigner<'a> {
        #[serde(rename = "keyId")]
        key_id: &'a str,
    }
    serde_json::to_vec(&Canonical {
        schema_version: attestation.schema_version,
        artifact: &attestation.artifact,
        identity: CanonicalIdentity {
            repository: &attestation.identity.repository,
            event: &attestation.identity.event,
            ref_name: &attestation.identity.ref_name,
            commit: &attestation.identity.commit,
            tree: &attestation.identity.tree,
            workflow: &attestation.identity.workflow,
            policy: &attestation.identity.policy,
            channel: &attestation.identity.channel,
            os: &attestation.identity.os,
        },
        signer: CanonicalSigner { key_id: &attestation.signer.key_id },
        update: if attestation.schema_version >= ATTESTATION_SCHEMA_V2 { attestation.update.as_ref() } else { None },
    })
    .map_err(|_| UpdateError::MalformedAttestation)
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn write_json_sync<T: Serialize>(path: &Path, value: &T) -> Result<(), UpdateError> {
    let data = serde_json::to_vec(value).map_err(|_| UpdateError::Filesystem)?;
    let mut file = File::create(path).map_err(|_| UpdateError::Filesystem)?;
    file.write_all(&data).map_err(|_| UpdateError::Filesystem)?;
    file.sync_all().map_err(|_| UpdateError::Filesystem)
}

fn remove_dir_if_exists(path: &Path) -> Result<(), UpdateError> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|_| UpdateError::Filesystem)?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<(), UpdateError> {
    if path.exists() {
        fs::remove_file(path).map_err(|_| UpdateError::Filesystem)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use ring::signature::KeyPair;

    fn manager(root: &Path) -> (UpdateManager, ring::signature::Ed25519KeyPair) {
        let rng = SystemRandom::new();
        let key = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let keypair = ring::signature::Ed25519KeyPair::from_pkcs8(key.as_ref()).unwrap();
        let policy = UpdatePolicy {
            channel: "stable".into(), os: "windows".into(), arch: "x86_64".into(),
            current_version: 3, minimum_version: 3, max_bytes: 1024, now: 100,
            repository: "stoltembergg-png/hank".into(), event: "release".into(),
            workflow: "release.yml".into(), policy: "updater-v1".into(),
            trusted_key_id: "fixture-v1".into(),
            trusted_public_key_der_b64: base64::engine::general_purpose::STANDARD.encode(keypair.public_key().as_ref()),
        };
        (UpdateManager::new(root, policy), keypair)
    }

    fn metadata(keypair: &ring::signature::Ed25519KeyPair) -> UpdateMetadata {
        let bytes = b"signed-update".to_vec();
        let artifact = ArtifactIdentity { name: "Hank.exe".into(), digest: digest(&bytes), size: bytes.len() as u64 };
        let identity = BuildIdentity {
            repository: "stoltembergg-png/hank".into(), event: "release".into(), ref_name: "refs/tags/v4".into(),
            commit: "a".repeat(40), tree: "b".repeat(40), workflow: "release.yml".into(), policy: "updater-v1".into(),
            channel: "stable".into(), os: "windows-x86_64".into(),
        };
        let signer = SignerIdentity { key_id: "fixture-v1".into() };
        let update = Some(UpdateBinding { version: 4, expires_at: 200, os: "windows".into(), arch: "x86_64".into() });
        let unsigned = UpdateAttestation { schema_version: 2, artifact, identity, signer, update, signature: SignatureEnvelope { algorithm: "ed25519".into(), value: String::new() } };
        let signature = keypair.sign(&canonical_payload(&unsigned).unwrap());
        let mut attestation = unsigned;
        attestation.signature.value = base64::engine::general_purpose::STANDARD.encode(signature.as_ref());
        UpdateMetadata { schema_version: 1, version: 4, channel: "stable".into(), os: "windows".into(), arch: "x86_64".into(), size: bytes.len() as u64, expires_at: 200, bytes, attestation }
    }

    #[test]
    fn signed_update_stages_and_rejects_tampering() {
        let root = std::env::temp_dir().join(format!("hank-updater-{}", uuid::Uuid::new_v4()));
        let (manager, keypair) = manager(&root);
        let mut update = metadata(&keypair);
        assert_eq!(manager.stage(&update, true).unwrap().version, 4);
        update.bytes = b"tampered".to_vec();
        update.size = update.bytes.len() as u64;
        assert_eq!(manager.stage(&update, true), Err(UpdateError::DigestMismatch));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn activation_keeps_previous_and_recovers_interruption() {
        let root = std::env::temp_dir().join(format!("hank-updater-{}", uuid::Uuid::new_v4()));
        let (manager, keypair) = manager(&root);
        let update = metadata(&keypair);
        manager.stage(&update, true).unwrap();
        fs::create_dir_all(root.join("current")).unwrap();
        fs::write(root.join("current/artifact.bin"), b"old").unwrap();
        manager.activate().unwrap();
        assert_eq!(fs::read(root.join("previous/artifact.bin")).unwrap(), b"old");
        fs::write(root.join("activation.pending"), b"pending").unwrap();
        fs::rename(root.join("current"), root.join("current-missing")).unwrap();
        assert_eq!(manager.recover_interrupted_activation().unwrap(), RecoveryResult::RestoredPrevious);
        assert!(root.join("current/artifact.bin").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn activation_rechecks_staged_digest_before_swapping_slots() {
        let root = std::env::temp_dir().join(format!("hank-updater-{}", uuid::Uuid::new_v4()));
        let (manager, keypair) = manager(&root);
        manager.stage(&metadata(&keypair), true).unwrap();
        fs::write(root.join("staging/artifact.bin"), b"tampered-after-stage").unwrap();
        assert_eq!(manager.activate(), Err(UpdateError::DigestMismatch));
        assert!(!root.join("current").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn consent_version_expiry_and_policy_fail_closed() {
        let root = std::env::temp_dir().join(format!("hank-updater-{}", uuid::Uuid::new_v4()));
        let (manager, keypair) = manager(&root);
        let update = metadata(&keypair);
        assert_eq!(manager.stage(&update, false), Err(UpdateError::ConsentRequired));
        let mut expired = update.clone(); expired.expires_at = 100;
        assert_eq!(manager.validate(&expired), Err(UpdateError::Expired));
        let mut wrong = update; wrong.channel = "beta".into();
        assert_eq!(manager.validate(&wrong), Err(UpdateError::PolicyMismatch));
        let _ = fs::remove_dir_all(root);
    }
}
