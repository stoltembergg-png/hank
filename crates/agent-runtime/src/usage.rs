//! Optional provider usage ledger with bounded, attempt-idempotent aggregation.

use agent_core::ids::{AgentId, ProjectId, SessionId};
use provider_core::{ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const USAGE_SCHEMA_VERSION: u32 = 1;
const MAX_ID_LEN: usize = 128;
const MAX_EVENTS: usize = 4096;
const MAX_TOKENS: u64 = 1_000_000_000;
const MAX_COST_MICROS: u64 = 1_000_000_000_000;
const MAX_CURRENCY_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSource {
    ProviderReported,
    Estimated,
    Missing,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageConfidence {
    Exact,
    Estimated,
    Unavailable,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageOutcome {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEvent {
    pub schema_version: u32,
    pub attempt_id: String,
    pub execution_id: String,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub provider_id: Option<ProviderId>,
    pub model_id: Option<ModelId>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_micros: Option<u64>,
    pub currency: Option<String>,
    pub source: UsageSource,
    pub confidence: UsageConfidence,
    pub outcome: UsageOutcome,
    pub terminal: bool,
}

impl UsageEvent {
    pub fn validate(&self) -> Result<(), UsageError> {
        if self.schema_version != USAGE_SCHEMA_VERSION
            || !valid_id(&self.attempt_id)
            || !valid_id(&self.execution_id)
        {
            return Err(UsageError::Invalid);
        }
        if !self.terminal {
            return Err(UsageError::NotTerminal);
        }
        if self.input_tokens.is_some_and(|value| value > MAX_TOKENS)
            || self.output_tokens.is_some_and(|value| value > MAX_TOKENS)
        {
            return Err(UsageError::Overflow);
        }
        if self
            .cost_micros
            .is_some_and(|value| value > MAX_COST_MICROS)
        {
            return Err(UsageError::Overflow);
        }
        if self.source == UsageSource::Missing {
            if self.input_tokens.is_some()
                || self.output_tokens.is_some()
                || self.cost_micros.is_some()
                || self.currency.is_some()
            {
                return Err(UsageError::Invalid);
            }
        } else if self.input_tokens.is_none()
            && self.output_tokens.is_none()
            && self.cost_micros.is_none()
        {
            return Err(UsageError::Invalid);
        }
        if self.cost_micros.is_some() != self.currency.is_some() {
            return Err(UsageError::Invalid);
        }
        if self.currency.as_deref().is_some_and(|currency| {
            currency.is_empty()
                || currency.len() > MAX_CURRENCY_LEN
                || !currency
                    .chars()
                    .all(|character| character.is_ascii_alphabetic())
        }) {
            return Err(UsageError::Invalid);
        }
        if self.provider_id.is_none() != self.model_id.is_none() {
            return Err(UsageError::Invalid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageReadModel {
    pub project_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_micros: Option<u64>,
    pub currency: Option<String>,
    pub currency_mismatch: bool,
    pub sample_count: u64,
    pub missing_usage_count: u64,
    pub source: UsageSource,
    pub confidence: UsageConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageRecordResult {
    Accepted,
    Duplicate,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum UsageError {
    #[error("usage event is invalid")]
    Invalid,
    #[error("usage event is not terminal")]
    NotTerminal,
    #[error("usage value overflowed bounded arithmetic")]
    Overflow,
    #[error("usage ledger capacity reached")]
    Capacity,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AggregateKey {
    project_id: String,
    agent_id: String,
    session_id: String,
}

#[derive(Debug, Clone)]
struct InternalAggregate {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost_micros: Option<u64>,
    currency: Option<String>,
    currency_mismatch: bool,
    sample_count: u64,
    missing_usage_count: u64,
    source: UsageSource,
    confidence: UsageConfidence,
}

pub struct UsageAggregator {
    max_events: usize,
    attempts: BTreeSet<String>,
    aggregates: BTreeMap<AggregateKey, InternalAggregate>,
}

impl UsageAggregator {
    pub fn new(max_events: usize) -> Result<Self, UsageError> {
        if !(1..=MAX_EVENTS).contains(&max_events) {
            return Err(UsageError::Capacity);
        }
        Ok(Self {
            max_events,
            attempts: BTreeSet::new(),
            aggregates: BTreeMap::new(),
        })
    }

    pub fn record(&mut self, event: UsageEvent) -> Result<UsageRecordResult, UsageError> {
        event.validate()?;
        if self.attempts.contains(&event.attempt_id) {
            return Ok(UsageRecordResult::Duplicate);
        }
        if self.attempts.len() >= self.max_events {
            return Err(UsageError::Capacity);
        }
        let key = AggregateKey {
            project_id: event.project_id.to_string(),
            agent_id: event.agent_id.to_string(),
            session_id: event.session_id.to_string(),
        };
        let mut aggregate = self
            .aggregates
            .get(&key)
            .cloned()
            .unwrap_or_else(|| InternalAggregate::new(&event));
        aggregate.merge(&event)?;
        self.attempts.insert(event.attempt_id);
        self.aggregates.insert(key, aggregate);
        Ok(UsageRecordResult::Accepted)
    }

    pub fn read_model(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Option<UsageReadModel> {
        let key = AggregateKey {
            project_id: project_id.to_string(),
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
        };
        self.aggregates.get(&key).map(|aggregate| UsageReadModel {
            project_id: key.project_id.clone(),
            agent_id: key.agent_id.clone(),
            session_id: key.session_id.clone(),
            input_tokens: aggregate.input_tokens,
            output_tokens: aggregate.output_tokens,
            cost_micros: aggregate.cost_micros,
            currency: aggregate.currency.clone(),
            currency_mismatch: aggregate.currency_mismatch,
            sample_count: aggregate.sample_count,
            missing_usage_count: aggregate.missing_usage_count,
            source: aggregate.source,
            confidence: aggregate.confidence,
        })
    }
}

impl InternalAggregate {
    fn new(event: &UsageEvent) -> Self {
        Self {
            input_tokens: None,
            output_tokens: None,
            cost_micros: None,
            currency: event.currency.clone(),
            currency_mismatch: false,
            sample_count: 0,
            missing_usage_count: 0,
            source: event.source,
            confidence: event.confidence,
        }
    }

    fn merge(&mut self, event: &UsageEvent) -> Result<(), UsageError> {
        self.input_tokens = checked_optional_add(self.input_tokens, event.input_tokens)?;
        self.output_tokens = checked_optional_add(self.output_tokens, event.output_tokens)?;
        self.sample_count = self
            .sample_count
            .checked_add(1)
            .ok_or(UsageError::Overflow)?;
        if event.source == UsageSource::Missing {
            self.missing_usage_count = self
                .missing_usage_count
                .checked_add(1)
                .ok_or(UsageError::Overflow)?;
        }
        self.source = combine_source(self.source, event.source);
        self.confidence = combine_confidence(self.confidence, event.confidence);
        if let Some(currency) = &event.currency {
            match &self.currency {
                None if !self.currency_mismatch => self.currency = Some(currency.clone()),
                Some(existing) if existing != currency => {
                    self.currency = None;
                    self.cost_micros = None;
                    self.currency_mismatch = true;
                }
                _ => {}
            }
            if !self.currency_mismatch {
                self.cost_micros = checked_optional_add(self.cost_micros, event.cost_micros)?;
            }
        }
        Ok(())
    }
}

fn checked_optional_add(left: Option<u64>, right: Option<u64>) -> Result<Option<u64>, UsageError> {
    match (left, right) {
        (Some(left), Some(right)) => left
            .checked_add(right)
            .map(Some)
            .ok_or(UsageError::Overflow),
        (Some(value), None) | (None, Some(value)) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

fn combine_source(left: UsageSource, right: UsageSource) -> UsageSource {
    if left == right {
        left
    } else {
        UsageSource::Mixed
    }
}

fn combine_confidence(left: UsageConfidence, right: UsageConfidence) -> UsageConfidence {
    if left == right {
        left
    } else {
        UsageConfidence::Mixed
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_LEN && !value.chars().any(char::is_control)
}
