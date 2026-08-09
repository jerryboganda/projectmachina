//! Fair scheduling and worker lease primitives.
//!
//! Queue selection rotates tenants while preserving priority and FIFO order
//! within each tenant. Worker leases expire explicitly and never become
//! available through a silent timeout.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueItem {
    pub id: String,
    pub tenant_id: String,
    pub priority: i32,
    pub enqueued_sequence: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FairQueue {
    tenants: BTreeMap<String, VecDeque<QueueItem>>,
    rotation: VecDeque<String>,
    next_sequence: u64,
}

impl FairQueue {
    pub fn enqueue(
        &mut self,
        id: impl Into<String>,
        tenant_id: impl Into<String>,
        priority: i32,
    ) -> Result<(), SchedulerError> {
        let id = id.into();
        let tenant_id = tenant_id.into();
        if id.trim().is_empty() || tenant_id.trim().is_empty() {
            return Err(SchedulerError::InvalidQueueItem);
        }
        if self
            .tenants
            .values()
            .any(|items| items.iter().any(|item| item.id == id))
        {
            return Err(SchedulerError::DuplicateQueueItem);
        }
        self.next_sequence += 1;
        let item = QueueItem {
            id,
            tenant_id: tenant_id.clone(),
            priority,
            enqueued_sequence: self.next_sequence,
        };
        let queue = self.tenants.entry(tenant_id.clone()).or_default();
        let insert_at = queue
            .iter()
            .position(|existing| {
                existing.priority < item.priority
                    || (existing.priority == item.priority
                        && existing.enqueued_sequence > item.enqueued_sequence)
            })
            .unwrap_or(queue.len());
        queue.insert(insert_at, item);
        if !self.rotation.contains(&tenant_id) {
            self.rotation.push_back(tenant_id);
        }
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<QueueItem> {
        let rounds = self.rotation.len();
        for _ in 0..rounds {
            let tenant = self.rotation.pop_front()?;
            let item = self.tenants.get_mut(&tenant).and_then(VecDeque::pop_front);
            if let Some(item) = item {
                if self
                    .tenants
                    .get(&tenant)
                    .is_some_and(|items| !items.is_empty())
                {
                    self.rotation.push_back(tenant);
                } else {
                    self.tenants.remove(&tenant);
                }
                return Some(item);
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.tenants.values().map(VecDeque::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Available,
    Busy,
    Draining,
    Offline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerRecord {
    pub worker_id: String,
    pub engine: String,
    pub capability_snapshot: String,
    pub state: WorkerState,
    pub lease_expires_at: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct WorkerRegistry {
    workers: BTreeMap<String, WorkerRecord>,
}

impl WorkerRegistry {
    pub fn register(
        &mut self,
        worker_id: impl Into<String>,
        engine: impl Into<String>,
        capability_snapshot: impl Into<String>,
    ) -> Result<WorkerRecord, SchedulerError> {
        let worker_id = worker_id.into();
        if worker_id.trim().is_empty() {
            return Err(SchedulerError::InvalidWorker);
        }
        if self.workers.contains_key(&worker_id) {
            return Err(SchedulerError::DuplicateWorker);
        }
        let worker = WorkerRecord {
            worker_id: worker_id.clone(),
            engine: engine.into(),
            capability_snapshot: capability_snapshot.into(),
            state: WorkerState::Available,
            lease_expires_at: None,
        };
        self.workers.insert(worker_id, worker.clone());
        Ok(worker)
    }

    pub fn acquire(
        &mut self,
        worker_id: &str,
        now: u64,
        lease_seconds: u64,
    ) -> Result<WorkerRecord, SchedulerError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(SchedulerError::WorkerNotFound)?;
        if worker.state != WorkerState::Available {
            return Err(SchedulerError::WorkerUnavailable);
        }
        worker.state = WorkerState::Busy;
        worker.lease_expires_at = Some(now.saturating_add(lease_seconds));
        Ok(worker.clone())
    }

    pub fn renew(
        &mut self,
        worker_id: &str,
        now: u64,
        lease_seconds: u64,
    ) -> Result<WorkerRecord, SchedulerError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(SchedulerError::WorkerNotFound)?;
        if worker.state != WorkerState::Busy
            || worker.lease_expires_at.is_none_or(|expiry| expiry <= now)
        {
            return Err(SchedulerError::LeaseExpired);
        }
        worker.lease_expires_at = Some(now.saturating_add(lease_seconds));
        Ok(worker.clone())
    }

    pub fn expire(&mut self, now: u64) -> Vec<String> {
        let mut expired = Vec::new();
        for worker in self.workers.values_mut() {
            if worker.state == WorkerState::Busy
                && worker.lease_expires_at.is_some_and(|expiry| expiry <= now)
            {
                worker.state = WorkerState::Offline;
                expired.push(worker.worker_id.clone());
            }
        }
        expired
    }

    pub fn release(&mut self, worker_id: &str) -> Result<WorkerRecord, SchedulerError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or(SchedulerError::WorkerNotFound)?;
        if worker.state != WorkerState::Busy {
            return Err(SchedulerError::WorkerUnavailable);
        }
        worker.state = WorkerState::Available;
        worker.lease_expires_at = None;
        Ok(worker.clone())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerError {
    InvalidQueueItem,
    DuplicateQueueItem,
    InvalidWorker,
    DuplicateWorker,
    WorkerNotFound,
    WorkerUnavailable,
    LeaseExpired,
}

impl Display for SchedulerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidQueueItem => "invalid queue item",
            Self::DuplicateQueueItem => "duplicate queue item",
            Self::InvalidWorker => "invalid worker",
            Self::DuplicateWorker => "duplicate worker",
            Self::WorkerNotFound => "worker not found",
            Self::WorkerUnavailable => "worker unavailable",
            Self::LeaseExpired => "worker lease expired",
        })
    }
}

impl std::error::Error for SchedulerError {}

#[cfg(test)]
mod tests {
    use super::{FairQueue, SchedulerError, WorkerRegistry, WorkerState};

    #[test]
    fn rotates_tenants_while_preserving_priority_within_tenant() {
        let mut queue = FairQueue::default();
        queue.enqueue("a-low", "tenant-a", 1).expect("enqueue");
        queue.enqueue("a-high", "tenant-a", 10).expect("enqueue");
        queue.enqueue("b", "tenant-b", 1).expect("enqueue");
        assert_eq!(
            queue.dequeue().map(|item| item.id),
            Some("a-high".to_owned())
        );
        assert_eq!(queue.dequeue().map(|item| item.id), Some("b".to_owned()));
        assert_eq!(
            queue.dequeue().map(|item| item.id),
            Some("a-low".to_owned())
        );
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn worker_leases_expire_and_cannot_renew_after_expiry() {
        let mut workers = WorkerRegistry::default();
        workers
            .register("worker-1", "native", "dom.query.v1")
            .expect("register");
        let acquired = workers.acquire("worker-1", 100, 10).expect("acquire");
        assert_eq!(acquired.state, WorkerState::Busy);
        assert_eq!(
            workers.renew("worker-1", 110, 10),
            Err(SchedulerError::LeaseExpired)
        );
        assert_eq!(workers.expire(110), vec!["worker-1".to_owned()]);
        assert_eq!(
            workers.release("worker-1"),
            Err(SchedulerError::WorkerUnavailable)
        );
    }
}
