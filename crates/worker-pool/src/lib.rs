//! Worker pool lifecycle and isolation contracts.
//!
//! This crate does not claim to launch Chromium. A runtime provider must
//! explicitly report prewarm success; unavailable runtime support is typed.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IsolationTier {
    SharedPerformance,
    DedicatedProcess,
    HardenedContainer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Cold,
    Prewarming,
    Ready,
    Leased,
    Draining,
    Offline,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerRecord {
    pub worker_id: String,
    pub engine: String,
    pub isolation: IsolationTier,
    pub capability_snapshot: String,
    pub generation: u64,
    pub state: WorkerState,
    pub lease_id: Option<String>,
    pub reset_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerError {
    InvalidWorker,
    WorkerNotFound,
    RuntimeUnavailable,
    WorkerNotReady,
    WorkerLeased,
    LeaseMismatch,
    Draining,
}

impl Display for WorkerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidWorker => "worker identity/configuration is invalid",
            Self::WorkerNotFound => "worker not found",
            Self::RuntimeUnavailable => "configured browser runtime is unavailable",
            Self::WorkerNotReady => "worker is not ready",
            Self::WorkerLeased => "worker is already leased",
            Self::LeaseMismatch => "worker lease does not match",
            Self::Draining => "worker is draining",
        })
    }
}

impl std::error::Error for WorkerError {}

#[derive(Clone, Debug, Default)]
pub struct WorkerPool {
    workers: BTreeMap<String, WorkerRecord>,
    next_lease: u64,
}

impl WorkerPool {
    pub fn register(
        &mut self,
        worker_id: impl Into<String>,
        engine: impl Into<String>,
        isolation: IsolationTier,
        capability_snapshot: impl Into<String>,
    ) -> Result<WorkerRecord, WorkerError> {
        let worker_id = worker_id.into();
        let engine = engine.into();
        if worker_id.trim().is_empty() || engine.trim().is_empty() {
            return Err(WorkerError::InvalidWorker);
        }
        if self.workers.contains_key(&worker_id) {
            return Err(WorkerError::InvalidWorker);
        }
        let record = WorkerRecord {
            worker_id: worker_id.clone(),
            engine,
            isolation,
            capability_snapshot: capability_snapshot.into(),
            generation: 0,
            state: WorkerState::Cold,
            lease_id: None,
            reset_count: 0,
        };
        self.workers.insert(worker_id, record.clone());
        Ok(record)
    }

    pub fn prewarm(
        &mut self,
        worker_id: &str,
        runtime_available: bool,
    ) -> Result<WorkerRecord, WorkerError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(WorkerError::WorkerNotFound)?;
        if !runtime_available {
            worker.state = WorkerState::Failed;
            return Err(WorkerError::RuntimeUnavailable);
        }
        if worker.state != WorkerState::Cold {
            return Err(WorkerError::WorkerNotReady);
        }
        worker.state = WorkerState::Ready;
        Ok(worker.clone())
    }

    pub fn lease(&mut self, worker_id: &str) -> Result<WorkerRecord, WorkerError> {
        self.next_lease += 1;
        let lease_id = format!("lease-{}", self.next_lease);
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(WorkerError::WorkerNotFound)?;
        if worker.state == WorkerState::Draining {
            return Err(WorkerError::Draining);
        }
        if worker.state != WorkerState::Ready {
            return Err(if worker.state == WorkerState::Leased {
                WorkerError::WorkerLeased
            } else {
                WorkerError::WorkerNotReady
            });
        }
        worker.state = WorkerState::Leased;
        worker.lease_id = Some(lease_id);
        Ok(worker.clone())
    }

    pub fn release(
        &mut self,
        worker_id: &str,
        lease_id: &str,
    ) -> Result<WorkerRecord, WorkerError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(WorkerError::WorkerNotFound)?;
        if !matches!(worker.state, WorkerState::Leased | WorkerState::Draining) {
            return Err(WorkerError::WorkerNotReady);
        }
        if worker.lease_id.as_deref() != Some(lease_id) {
            return Err(WorkerError::LeaseMismatch);
        }
        worker.state = if worker.state == WorkerState::Draining {
            WorkerState::Offline
        } else {
            WorkerState::Ready
        };
        worker.lease_id = None;
        Ok(worker.clone())
    }

    pub fn reset(&mut self, worker_id: &str) -> Result<WorkerRecord, WorkerError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(WorkerError::WorkerNotFound)?;
        if worker.state != WorkerState::Ready {
            return Err(match worker.state {
                WorkerState::Leased => WorkerError::WorkerLeased,
                WorkerState::Draining => WorkerError::Draining,
                _ => WorkerError::WorkerNotReady,
            });
        }
        worker.generation += 1;
        worker.reset_count += 1;
        worker.lease_id = None;
        worker.state = WorkerState::Ready;
        Ok(worker.clone())
    }

    pub fn drain(&mut self, worker_id: &str) -> Result<WorkerRecord, WorkerError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(WorkerError::WorkerNotFound)?;
        if !matches!(worker.state, WorkerState::Ready | WorkerState::Leased) {
            return Err(WorkerError::WorkerNotReady);
        }
        worker.state = WorkerState::Draining;
        Ok(worker.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{IsolationTier, WorkerError, WorkerPool, WorkerState};

    #[test]
    fn unavailable_runtime_never_reports_ready() {
        let mut pool = WorkerPool::default();
        pool.register(
            "chromium-1",
            "chromium",
            IsolationTier::DedicatedProcess,
            "visual.v1",
        )
        .expect("register");
        assert_eq!(
            pool.prewarm("chromium-1", false),
            Err(WorkerError::RuntimeUnavailable)
        );
        assert_eq!(pool.lease("chromium-1"), Err(WorkerError::WorkerNotReady));
    }

    #[test]
    fn lease_generation_and_reset_are_explicit() {
        let mut pool = WorkerPool::default();
        pool.register(
            "native-1",
            "native",
            IsolationTier::SharedPerformance,
            "dom.v1",
        )
        .expect("register");
        pool.prewarm("native-1", true).expect("prewarm");
        let leased = pool.lease("native-1").expect("lease");
        let lease_id = leased.lease_id.clone().expect("lease id");
        assert_eq!(
            pool.release("native-1", "stale"),
            Err(WorkerError::LeaseMismatch)
        );
        assert_eq!(pool.reset("native-1"), Err(WorkerError::WorkerLeased));
        pool.release("native-1", &lease_id).expect("release");
        let reset = pool.reset("native-1").expect("reset");
        assert_eq!(reset.generation, 1);
        assert_eq!(reset.state, WorkerState::Ready);
        pool.drain("native-1").expect("drain");
        assert_eq!(pool.lease("native-1"), Err(WorkerError::Draining));
    }
}
