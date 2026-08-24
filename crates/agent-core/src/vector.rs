//! Optional local vector retrieval backend over typed embedding records.

use crate::ids::ProjectId;
use crate::taxonomy::MemoryKind;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct VectorRecord {
    pub id: String,
    pub project_id: ProjectId,
    pub agent_id: Option<String>,
    pub kind: MemoryKind,
    pub model: String,
    pub model_version: String,
    pub vector: Vec<f32>,
    pub content_ref: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct VectorQuery {
    pub project_id: ProjectId,
    pub agent_id: Option<String>,
    pub model: String,
    pub model_version: String,
    pub vector: Vec<f32>,
    pub k: usize,
    pub max_bytes: usize,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorResult {
    pub id: String,
    pub content_ref: String,
    pub similarity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum VectorError {
    #[error("vector record is invalid")]
    InvalidRecord,
    #[error("vector identity is duplicated across scopes")]
    DuplicateIdentity,
    #[error("vector dimensions do not match")]
    DimensionMismatch,
    #[error("vector model identity does not match")]
    ModelMismatch,
    #[error("vector record belongs to another project")]
    ProjectScope,
    #[error("vector query is invalid")]
    InvalidQuery,
    #[error("vector record is not found")]
    NotFound,
}

#[derive(Debug, Default)]
pub struct VectorIndex {
    records: Vec<VectorRecord>,
}

impl VectorIndex {
    pub fn upsert(&mut self, record: VectorRecord) -> Result<(), VectorError> {
        validate_record(&record)?;
        if let Some(existing) = self.records.iter_mut().find(|entry| entry.id == record.id) {
            if existing.project_id != record.project_id {
                return Err(VectorError::DuplicateIdentity);
            }
            *existing = record;
        } else {
            self.records.push(record);
        }
        Ok(())
    }

    pub fn query(&self, query: &VectorQuery) -> Result<Vec<VectorResult>, VectorError> {
        if query.vector.is_empty()
            || query.k == 0
            || query.k > 100
            || query.max_bytes == 0
            || query.model.is_empty()
            || query.model_version.is_empty()
            || query.trace_id.is_empty()
        {
            return Err(VectorError::InvalidQuery);
        }
        let mut results = self
            .records
            .iter()
            .filter(|record| {
                record.active
                    && record.project_id == query.project_id
                    && record.agent_id == query.agent_id
                    && record.model == query.model
                    && record.model_version == query.model_version
            })
            .map(|record| {
                if record.vector.len() != query.vector.len() {
                    return Err(VectorError::DimensionMismatch);
                }
                Ok(VectorResult {
                    id: record.id.clone(),
                    content_ref: record.content_ref.clone(),
                    similarity: cosine(&record.vector, &query.vector),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        results.sort_by(|left, right| {
            right
                .similarity
                .total_cmp(&left.similarity)
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut bytes = 0;
        results.retain(|result| {
            let size = result.content_ref.len();
            if bytes + size > query.max_bytes {
                return false;
            }
            bytes += size;
            true
        });
        results.truncate(query.k);
        Ok(results)
    }

    pub fn archive(&mut self, project_id: &ProjectId, id: &str) -> Result<(), VectorError> {
        let record = self
            .records
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(VectorError::NotFound)?;
        if &record.project_id != project_id {
            return Err(VectorError::ProjectScope);
        }
        record.active = false;
        Ok(())
    }

    pub fn rebuild(&mut self, records: Vec<VectorRecord>) -> Result<(), VectorError> {
        let baseline = self.records.first().or_else(|| records.first());
        let expected_dimensions = baseline.map(|record| record.vector.len());
        let expected_model =
            baseline.map(|record| (record.model.clone(), record.model_version.clone()));
        for record in &records {
            validate_record(record)?;
            if Some(record.vector.len()) != expected_dimensions
                || expected_model.as_ref()
                    != Some(&(record.model.clone(), record.model_version.clone()))
            {
                return Err(VectorError::DimensionMismatch);
            }
        }
        let mut rebuilt = Self::default();
        for record in records {
            rebuilt.upsert(record)?;
        }
        self.records = rebuilt.records;
        Ok(())
    }
}

fn validate_record(record: &VectorRecord) -> Result<(), VectorError> {
    if record.id.is_empty()
        || record.model.is_empty()
        || record.model_version.is_empty()
        || record.vector.is_empty()
        || record.vector.len() > 4096
        || record.vector.iter().any(|value| !value.is_finite())
        || record.content_ref.is_empty()
        || record.content_ref.len() > 256
    {
        return Err(VectorError::InvalidRecord);
    }
    Ok(())
}

fn cosine(left: &[f32], right: &[f32]) -> f32 {
    let dot: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm * right_norm)
    }
}
