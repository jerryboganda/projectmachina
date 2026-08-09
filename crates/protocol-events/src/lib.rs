//! Ordered event stream primitives shared by HTTP/gRPC and future adapters.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamEvent {
    pub sequence: u64,
    pub event_type: String,
    pub payload: String,
}

impl StreamEvent {
    pub fn event_id(&self, session_id: &str) -> Result<String, StreamError> {
        if session_id.trim().is_empty() {
            return Err(StreamError::InvalidSession);
        }
        Ok(format!("{session_id}:{}", self.sequence))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamError {
    InvalidEvent,
    InvalidSession,
    Backpressure,
    ResumeOutOfRange { requested: u64, oldest: u64 },
}

impl Display for StreamError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEvent => formatter.write_str("event type and payload are required"),
            Self::InvalidSession => formatter.write_str("session ID is required"),
            Self::Backpressure => formatter.write_str("event stream buffer is full"),
            Self::ResumeOutOfRange { requested, oldest } => {
                write!(
                    formatter,
                    "resume sequence {requested} is older than {oldest}"
                )
            }
        }
    }
}

impl std::error::Error for StreamError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResumeResult {
    Events(Vec<StreamEvent>),
    ResyncRequired { requested: u64, oldest: u64 },
}

#[derive(Clone, Debug)]
pub struct EventStream {
    capacity: usize,
    next_sequence: u64,
    acknowledged: u64,
    events: VecDeque<StreamEvent>,
}

impl EventStream {
    pub fn new(capacity: usize) -> Result<Self, StreamError> {
        if capacity == 0 {
            return Err(StreamError::InvalidEvent);
        }
        Ok(Self {
            capacity,
            next_sequence: 0,
            acknowledged: 0,
            events: VecDeque::new(),
        })
    }

    pub fn publish(
        &mut self,
        event_type: impl Into<String>,
        payload: impl Into<String>,
    ) -> Result<StreamEvent, StreamError> {
        let event_type = event_type.into();
        let payload = payload.into();
        if event_type.trim().is_empty() || payload.trim().is_empty() {
            return Err(StreamError::InvalidEvent);
        }
        if self.events.len() >= self.capacity {
            return Err(StreamError::Backpressure);
        }
        self.next_sequence += 1;
        let event = StreamEvent {
            sequence: self.next_sequence,
            event_type,
            payload,
        };
        self.events.push_back(event.clone());
        Ok(event)
    }

    pub fn resume_after(&self, sequence: u64) -> Result<Vec<StreamEvent>, StreamError> {
        match self.resume(sequence) {
            ResumeResult::Events(events) => Ok(events),
            ResumeResult::ResyncRequired { requested, oldest } => {
                Err(StreamError::ResumeOutOfRange { requested, oldest })
            }
        }
    }

    pub fn resume(&self, sequence: u64) -> ResumeResult {
        let oldest = self
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or(sequence.saturating_add(1));
        if !self.events.is_empty() && sequence.saturating_add(1) < oldest {
            return ResumeResult::ResyncRequired {
                requested: sequence,
                oldest,
            };
        }
        ResumeResult::Events(
            self.events
                .iter()
                .filter(|event| event.sequence > sequence)
                .cloned()
                .collect(),
        )
    }

    pub fn acknowledge(&mut self, sequence: u64) {
        self.acknowledged = self.acknowledged.max(sequence);
        while self
            .events
            .front()
            .is_some_and(|event| event.sequence <= self.acknowledged)
        {
            self.events.pop_front();
        }
    }

    pub fn acknowledged(&self) -> u64 {
        self.acknowledged
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrokerError {
    InvalidSession,
    InvalidSubscriber,
    InvalidCapacity,
    SessionNotFound,
    SubscriberNotFound,
    SubscriberAlreadyExists,
    InvalidAcknowledgement { requested: u64, latest: u64 },
    Stream(StreamError),
}

impl Display for BrokerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSession => formatter.write_str("session ID is required"),
            Self::InvalidSubscriber => formatter.write_str("subscriber ID is required"),
            Self::InvalidCapacity => formatter.write_str("buffer capacity must be positive"),
            Self::SessionNotFound => formatter.write_str("event session is not registered"),
            Self::SubscriberNotFound => formatter.write_str("event subscriber is not registered"),
            Self::SubscriberAlreadyExists => formatter.write_str("event subscriber already exists"),
            Self::InvalidAcknowledgement { requested, latest } => {
                write!(
                    formatter,
                    "acknowledgement {requested} exceeds latest event {latest}"
                )
            }
            Self::Stream(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for BrokerError {}

impl From<StreamError> for BrokerError {
    fn from(error: StreamError) -> Self {
        Self::Stream(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Subscriber {
    capacity: usize,
    pending: VecDeque<StreamEvent>,
    acknowledged: u64,
    resync_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SessionChannel {
    capacity: usize,
    next_sequence: u64,
    history: VecDeque<StreamEvent>,
    subscribers: BTreeMap<String, Subscriber>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishReport {
    pub event: StreamEvent,
    pub resync_required: Vec<String>,
}

/// A bounded, per-session event broker. History and each subscriber queue have
/// independent limits so a slow reader cannot grow process memory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventBroker {
    history_capacity: usize,
    sessions: BTreeMap<String, SessionChannel>,
}

impl EventBroker {
    pub fn new(history_capacity: usize) -> Result<Self, BrokerError> {
        if history_capacity == 0 {
            return Err(BrokerError::InvalidCapacity);
        }
        Ok(Self {
            history_capacity,
            sessions: BTreeMap::new(),
        })
    }

    pub fn register_session(&mut self, session_id: impl Into<String>) -> Result<(), BrokerError> {
        let session_id = session_id.into();
        if session_id.trim().is_empty() {
            return Err(BrokerError::InvalidSession);
        }
        self.sessions
            .entry(session_id)
            .or_insert_with(|| SessionChannel {
                capacity: self.history_capacity,
                next_sequence: 0,
                history: VecDeque::new(),
                subscribers: BTreeMap::new(),
            });
        Ok(())
    }

    pub fn subscribe(
        &mut self,
        session_id: &str,
        subscriber_id: impl Into<String>,
        capacity: usize,
    ) -> Result<(), BrokerError> {
        if session_id.trim().is_empty() {
            return Err(BrokerError::InvalidSession);
        }
        if capacity == 0 {
            return Err(BrokerError::InvalidCapacity);
        }
        let subscriber_id = subscriber_id.into();
        if subscriber_id.trim().is_empty() {
            return Err(BrokerError::InvalidSubscriber);
        }
        let channel = self
            .sessions
            .get_mut(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        if channel.subscribers.contains_key(&subscriber_id) {
            return Err(BrokerError::SubscriberAlreadyExists);
        }
        channel.subscribers.insert(
            subscriber_id,
            Subscriber {
                capacity,
                pending: VecDeque::new(),
                acknowledged: 0,
                resync_required: false,
            },
        );
        Ok(())
    }

    pub fn unsubscribe(
        &mut self,
        session_id: &str,
        subscriber_id: &str,
    ) -> Result<(), BrokerError> {
        let channel = self
            .sessions
            .get_mut(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        if channel.subscribers.remove(subscriber_id).is_none() {
            return Err(BrokerError::SubscriberNotFound);
        }
        Ok(())
    }

    /// Complete a client-provided snapshot/resync and resume delivery for the
    /// existing subscriber without allocating an additional queue.
    pub fn reset_subscriber(
        &mut self,
        session_id: &str,
        subscriber_id: &str,
        sequence: u64,
    ) -> Result<ResumeResult, BrokerError> {
        let (events, capacity) = {
            let channel = self
                .sessions
                .get(session_id)
                .ok_or(BrokerError::SessionNotFound)?;
            let subscriber = channel
                .subscribers
                .get(subscriber_id)
                .ok_or(BrokerError::SubscriberNotFound)?;
            if sequence > channel.next_sequence {
                return Err(BrokerError::InvalidAcknowledgement {
                    requested: sequence,
                    latest: channel.next_sequence,
                });
            }
            let oldest = channel
                .history
                .front()
                .map(|event| event.sequence)
                .unwrap_or(sequence.saturating_add(1));
            if !channel.history.is_empty() && sequence.saturating_add(1) < oldest {
                return Ok(ResumeResult::ResyncRequired {
                    requested: sequence,
                    oldest,
                });
            }
            (
                channel
                    .history
                    .iter()
                    .filter(|event| event.sequence > sequence)
                    .cloned()
                    .collect::<Vec<_>>(),
                subscriber.capacity,
            )
        };
        if events.len() > capacity {
            let oldest = events
                .first()
                .map(|event| event.sequence)
                .unwrap_or(sequence.saturating_add(1));
            return Ok(ResumeResult::ResyncRequired {
                requested: sequence,
                oldest,
            });
        }
        let channel = self
            .sessions
            .get_mut(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        let subscriber = channel
            .subscribers
            .get_mut(subscriber_id)
            .ok_or(BrokerError::SubscriberNotFound)?;
        subscriber.pending = events.iter().cloned().collect();
        subscriber.acknowledged = sequence;
        subscriber.resync_required = false;
        Ok(ResumeResult::Events(events))
    }

    pub fn publish(
        &mut self,
        session_id: &str,
        event_type: impl Into<String>,
        payload: impl Into<String>,
    ) -> Result<PublishReport, BrokerError> {
        let channel = self
            .sessions
            .get_mut(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        let event_type = event_type.into();
        let payload = payload.into();
        if event_type.trim().is_empty() || payload.trim().is_empty() {
            return Err(BrokerError::Stream(StreamError::InvalidEvent));
        }

        channel.next_sequence += 1;
        let event = StreamEvent {
            sequence: channel.next_sequence,
            event_type,
            payload,
        };
        channel.history.push_back(event.clone());
        while channel.history.len() > channel.capacity {
            channel.history.pop_front();
        }

        let mut resync_required = Vec::new();
        for (subscriber_id, subscriber) in &mut channel.subscribers {
            if subscriber.resync_required {
                continue;
            }
            if subscriber.pending.len() >= subscriber.capacity {
                subscriber.pending.clear();
                subscriber.resync_required = true;
                resync_required.push(subscriber_id.clone());
                continue;
            }
            subscriber.pending.push_back(event.clone());
        }

        Ok(PublishReport {
            event,
            resync_required,
        })
    }

    pub fn poll(&self, session_id: &str, subscriber_id: &str) -> Result<ResumeResult, BrokerError> {
        let channel = self
            .sessions
            .get(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        let subscriber = channel
            .subscribers
            .get(subscriber_id)
            .ok_or(BrokerError::SubscriberNotFound)?;
        if subscriber.resync_required {
            return Ok(self.resync_result(channel, subscriber.acknowledged));
        }
        Ok(ResumeResult::Events(
            subscriber.pending.iter().cloned().collect(),
        ))
    }

    pub fn resume_after(
        &self,
        session_id: &str,
        subscriber_id: &str,
        sequence: u64,
    ) -> Result<ResumeResult, BrokerError> {
        let channel = self
            .sessions
            .get(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        let subscriber = channel
            .subscribers
            .get(subscriber_id)
            .ok_or(BrokerError::SubscriberNotFound)?;
        if sequence > channel.next_sequence {
            return Err(BrokerError::InvalidAcknowledgement {
                requested: sequence,
                latest: channel.next_sequence,
            });
        }
        if subscriber.resync_required {
            return Ok(self.resync_result(channel, sequence));
        }
        let oldest = channel
            .history
            .front()
            .map(|event| event.sequence)
            .unwrap_or(sequence.saturating_add(1));
        if !channel.history.is_empty() && sequence.saturating_add(1) < oldest {
            return Ok(ResumeResult::ResyncRequired {
                requested: sequence,
                oldest,
            });
        }
        Ok(ResumeResult::Events(
            channel
                .history
                .iter()
                .filter(|event| event.sequence > sequence)
                .cloned()
                .collect(),
        ))
    }

    pub fn acknowledge(
        &mut self,
        session_id: &str,
        subscriber_id: &str,
        sequence: u64,
    ) -> Result<(), BrokerError> {
        let channel = self
            .sessions
            .get_mut(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        if sequence > channel.next_sequence {
            return Err(BrokerError::InvalidAcknowledgement {
                requested: sequence,
                latest: channel.next_sequence,
            });
        }
        let subscriber = channel
            .subscribers
            .get_mut(subscriber_id)
            .ok_or(BrokerError::SubscriberNotFound)?;
        subscriber.acknowledged = subscriber.acknowledged.max(sequence);
        while subscriber
            .pending
            .front()
            .is_some_and(|event| event.sequence <= subscriber.acknowledged)
        {
            subscriber.pending.pop_front();
        }
        Ok(())
    }

    pub fn pending_len(&self, session_id: &str, subscriber_id: &str) -> Result<usize, BrokerError> {
        let channel = self
            .sessions
            .get(session_id)
            .ok_or(BrokerError::SessionNotFound)?;
        let subscriber = channel
            .subscribers
            .get(subscriber_id)
            .ok_or(BrokerError::SubscriberNotFound)?;
        Ok(subscriber.pending.len())
    }

    fn resync_result(&self, channel: &SessionChannel, requested: u64) -> ResumeResult {
        ResumeResult::ResyncRequired {
            requested,
            oldest: channel
                .history
                .front()
                .map(|event| event.sequence)
                .unwrap_or(channel.next_sequence.saturating_add(1)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumeResult {
    Applied,
    Duplicate,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ConsumerError<E> {
    InvalidEventId,
    Apply(E),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IdempotentConsumer {
    applied_event_ids: BTreeSet<String>,
}

impl IdempotentConsumer {
    pub fn consume<F, E>(
        &mut self,
        event_id: &str,
        apply: F,
    ) -> Result<ConsumeResult, ConsumerError<E>>
    where
        F: FnOnce() -> Result<(), E>,
    {
        if event_id.trim().is_empty() {
            return Err(ConsumerError::InvalidEventId);
        }
        if self.applied_event_ids.contains(event_id) {
            return Ok(ConsumeResult::Duplicate);
        }
        apply().map_err(ConsumerError::Apply)?;
        self.applied_event_ids.insert(event_id.to_owned());
        Ok(ConsumeResult::Applied)
    }

    pub fn contains(&self, event_id: &str) -> bool {
        self.applied_event_ids.contains(event_id)
    }

    pub fn applied_count(&self) -> usize {
        self.applied_event_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConsumeResult, EventBroker, EventStream, IdempotentConsumer, ResumeResult, StreamError,
    };

    #[test]
    fn preserves_order_resume_and_backpressure() {
        let mut stream = EventStream::new(2).expect("stream");
        let first = stream.publish("session.ready.v1", "{}").expect("event");
        assert_eq!(
            first.event_id("session-1").expect("event ID"),
            "session-1:1"
        );
        assert_eq!(first.event_id(""), Err(StreamError::InvalidSession));
        stream.publish("session.closed.v1", "{}").expect("event");
        assert_eq!(
            stream.publish("session.extra.v1", "{}"),
            Err(StreamError::Backpressure)
        );
        assert_eq!(
            stream.resume_after(first.sequence).expect("resume").len(),
            1
        );
        stream.acknowledge(first.sequence);
        assert_eq!(stream.acknowledged(), first.sequence);
        stream
            .publish("session.extra.v1", "{}")
            .expect("capacity freed");
    }

    #[test]
    fn reports_resync_when_history_gap_is_older_than_buffer() {
        let mut broker = EventBroker::new(2).expect("broker");
        broker.register_session("session-1").expect("session");
        broker
            .subscribe("session-1", "reader-1", 2)
            .expect("subscribe");
        broker
            .publish("session-1", "session.requested.v1", "{}")
            .expect("event");
        broker
            .publish("session-1", "session.ready.v1", "{}")
            .expect("event");
        broker
            .publish("session-1", "session.closed.v1", "{}")
            .expect("event");

        assert_eq!(
            broker
                .resume_after("session-1", "reader-1", 0)
                .expect("resume"),
            ResumeResult::ResyncRequired {
                requested: 0,
                oldest: 2
            }
        );
    }

    #[test]
    fn bounds_slow_subscriber_and_marks_resync() {
        let mut broker = EventBroker::new(8).expect("broker");
        broker.register_session("session-1").expect("session");
        broker.subscribe("session-1", "slow", 2).expect("subscribe");
        broker
            .publish("session-1", "session.requested.v1", "{}")
            .expect("event");
        broker
            .publish("session-1", "session.ready.v1", "{}")
            .expect("event");
        let report = broker
            .publish("session-1", "session.closed.v1", "{}")
            .expect("event");

        assert_eq!(report.resync_required, vec!["slow".to_owned()]);
        assert_eq!(broker.pending_len("session-1", "slow").expect("pending"), 0);
        assert_eq!(
            broker.poll("session-1", "slow").expect("poll"),
            ResumeResult::ResyncRequired {
                requested: 0,
                oldest: 1
            }
        );
    }

    #[test]
    fn resync_reset_reactivates_slow_subscriber() {
        let mut broker = EventBroker::new(8).expect("broker");
        broker.register_session("session-1").expect("session");
        broker.subscribe("session-1", "slow", 2).expect("subscribe");
        broker
            .publish("session-1", "session.requested.v1", "{}")
            .expect("event");
        broker
            .publish("session-1", "session.ready.v1", "{}")
            .expect("event");
        broker
            .publish("session-1", "session.closed.v1", "{}")
            .expect("event");
        assert!(matches!(
            broker.poll("session-1", "slow").expect("poll"),
            ResumeResult::ResyncRequired { .. }
        ));

        assert_eq!(
            broker
                .reset_subscriber("session-1", "slow", 3)
                .expect("reset"),
            ResumeResult::Events(Vec::new())
        );
        broker
            .publish("session-1", "session.archived.v1", "{}")
            .expect("event");
        assert!(matches!(
            broker.poll("session-1", "slow").expect("poll"),
            ResumeResult::Events(events) if events.len() == 1 && events[0].sequence == 4
        ));
    }

    #[test]
    fn acknowledges_delivery_and_replays_in_order() {
        let mut broker = EventBroker::new(4).expect("broker");
        broker.register_session("session-1").expect("session");
        broker
            .subscribe("session-1", "reader-1", 4)
            .expect("subscribe");
        let first = broker
            .publish("session-1", "session.requested.v1", "{}")
            .expect("event")
            .event;
        broker
            .publish("session-1", "session.ready.v1", "{}")
            .expect("event");

        broker
            .acknowledge("session-1", "reader-1", first.sequence)
            .expect("acknowledge");
        let pending = broker.poll("session-1", "reader-1").expect("poll");
        assert_eq!(
            pending,
            ResumeResult::Events(vec![super::StreamEvent {
                sequence: 2,
                event_type: "session.ready.v1".to_owned(),
                payload: "{}".to_owned()
            }])
        );
    }

    #[test]
    fn duplicate_delivery_does_not_repeat_durable_effect() {
        let mut consumer = IdempotentConsumer::default();
        let mut applied = 0;
        assert_eq!(
            consumer
                .consume("event-1", || {
                    applied += 1;
                    Ok::<(), ()>(())
                })
                .expect("apply"),
            ConsumeResult::Applied
        );
        assert_eq!(
            consumer
                .consume("event-1", || {
                    applied += 1;
                    Ok::<(), ()>(())
                })
                .expect("duplicate"),
            ConsumeResult::Duplicate
        );
        assert_eq!(applied, 1);
        assert_eq!(consumer.applied_count(), 1);
        assert!(consumer.contains("event-1"));
    }

    #[test]
    fn rejects_missing_event_id_before_applying_effect() {
        let mut consumer = IdempotentConsumer::default();
        let mut applied = false;
        assert_eq!(
            consumer.consume("", || {
                applied = true;
                Ok::<(), ()>(())
            }),
            Err(super::ConsumerError::InvalidEventId)
        );
        assert!(!applied);
    }
}
