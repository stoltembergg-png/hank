//! Bounded keyword retrieval independent from vector backends.

use crate::ids::ProjectId;
use crate::memory::MemoryStatus;
use crate::taxonomy::MemoryKind;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct KeywordMemoryRecord {
    pub id: String,
    pub project_id: ProjectId,
    pub agent_id: Option<String>,
    pub kind: MemoryKind,
    pub status: MemoryStatus,
    pub content: String,
    pub importance: f32,
}

#[derive(Debug, Clone)]
pub struct KeywordQuery {
    pub project_id: ProjectId,
    pub agent_id: Option<String>,
    pub terms: String,
    pub max_results: usize,
    pub max_bytes: usize,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeywordResult {
    pub id: String,
    pub content: String,
    pub match_count: usize,
    pub importance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum KeywordError {
    #[error("keyword input is invalid")]
    InvalidInput,
    #[error("keyword identity is duplicated")]
    DuplicateIdentity,
}

#[derive(Debug, Default)]
pub struct KeywordRetriever {
    records: Vec<KeywordMemoryRecord>,
}

impl KeywordRetriever {
    pub fn insert(&mut self, record: KeywordMemoryRecord) -> Result<(), KeywordError> {
        if record.id.is_empty()
            || record.id.len() > 128
            || record.content.trim().is_empty()
            || record.content.len() > 16 * 1024
            || !record.importance.is_finite()
            || !(0.0..=1.0).contains(&record.importance)
        {
            return Err(KeywordError::InvalidInput);
        }
        if self.records.iter().any(|existing| existing.id == record.id) {
            return Err(KeywordError::DuplicateIdentity);
        }
        self.records.push(record);
        Ok(())
    }

    pub fn query(&self, query: &KeywordQuery) -> Result<Vec<KeywordResult>, KeywordError> {
        if query.terms.len() > 4096
            || query.terms.trim().is_empty()
            || query.trace_id.is_empty()
            || query.max_results == 0
            || query.max_results > 100
            || query.max_bytes == 0
        {
            return Err(KeywordError::InvalidInput);
        }
        let terms = tokens(&query.terms);
        if terms.is_empty() || terms.len() > 64 {
            return Err(KeywordError::InvalidInput);
        }
        let mut matches = self
            .records
            .iter()
            .filter(|record| {
                record.project_id == query.project_id
                    && record.agent_id == query.agent_id
                    && record.status == MemoryStatus::Approved
            })
            .filter_map(|record| {
                let words = tokens(&record.content);
                let count = terms.iter().filter(|term| words.contains(term)).count();
                (count > 0).then_some((record, count))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|(left, left_count), (right, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| right.importance.total_cmp(&left.importance))
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut used = 0;
        let mut results = Vec::new();
        for (record, count) in matches {
            if results.len() >= query.max_results {
                break;
            }
            if used + record.content.len() > query.max_bytes {
                continue;
            }
            used += record.content.len();
            results.push(KeywordResult {
                id: record.id.clone(),
                content: record.content.clone(),
                match_count: count,
                importance: record.importance,
            });
        }
        Ok(results)
    }
}

fn tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}
