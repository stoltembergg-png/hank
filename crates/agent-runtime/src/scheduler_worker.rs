use crate::event_bus::{EventBus, EventBusError};
use crate::scheduler_persistence::{SchedulerPersistence, SchedulerRun};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchEnvelope {
    pub project_id: String,
    pub run_id: String,
    pub job_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WorkerError {
    #[error("scheduler worker is stopped")]
    Stopped,
    #[error("scheduler dispatch bus is unavailable")]
    DispatchUnavailable,
    #[error("scheduler persistence failed")]
    Persistence,
}

pub struct SchedulerWorker {
    persistence: SchedulerPersistence,
    dispatch: EventBus<DispatchEnvelope>,
    owner_id: String,
    lease_duration_ms: u64,
    max_claims_per_tick: u32,
    stopped: Arc<AtomicBool>,
}

impl SchedulerWorker {
    pub fn new(
        persistence: SchedulerPersistence,
        dispatch: EventBus<DispatchEnvelope>,
        owner_id: &str,
        lease_duration_ms: u64,
        max_claims_per_tick: u32,
    ) -> Result<Self, WorkerError> {
        if owner_id.is_empty()
            || lease_duration_ms == 0
            || max_claims_per_tick == 0
            || max_claims_per_tick > 64
        {
            return Err(WorkerError::Persistence);
        }
        Ok(Self {
            persistence,
            dispatch,
            owner_id: owner_id.into(),
            lease_duration_ms,
            max_claims_per_tick,
            stopped: Arc::new(AtomicBool::new(false)),
        })
    }

    pub async fn tick(&self, project: &str, now_ms: u64) -> Result<u32, WorkerError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(WorkerError::Stopped);
        }
        let mut dispatched = 0;
        for _ in 0..self.max_claims_per_tick {
            let Some(run) = self
                .persistence
                .claim_next_due(project, &self.owner_id, now_ms, self.lease_duration_ms)
                .await
                .map_err(|_| WorkerError::Persistence)?
            else {
                break;
            };
            let envelope = envelope(&run);
            self.dispatch
                .publish(envelope)
                .map_err(|error| match error {
                    EventBusError::Closed | EventBusError::NoSubscribers => {
                        WorkerError::DispatchUnavailable
                    }
                    EventBusError::Lagged(_) => WorkerError::DispatchUnavailable,
                })?;
            dispatched += 1;
        }
        Ok(dispatched)
    }

    pub async fn renew(
        &self,
        run: &SchedulerRun,
        now_ms: u64,
    ) -> Result<SchedulerRun, WorkerError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(WorkerError::Stopped);
        }
        self.persistence
            .renew(
                &run.project_id,
                &run.run_id,
                &self.owner_id,
                now_ms,
                self.lease_duration_ms,
            )
            .await
            .map_err(|_| WorkerError::Persistence)
    }

    pub fn shutdown(&self) {
        self.stopped.store(true, Ordering::Release);
    }
}

fn envelope(run: &SchedulerRun) -> DispatchEnvelope {
    DispatchEnvelope {
        project_id: run.project_id.clone(),
        run_id: run.run_id.clone(),
        job_id: run.job_id.clone(),
        idempotency_key: format!("scheduler:{}:{}", run.project_id, run.run_id),
    }
}
