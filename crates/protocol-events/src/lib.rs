//! Ordered event stream primitives shared by HTTP/gRPC and future adapters.

use std::collections::VecDeque;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamEvent {
    pub sequence: u64,
    pub event_type: String,
    pub payload: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamError {
    InvalidEvent,
    Backpressure,
    ResumeOutOfRange { requested: u64, oldest: u64 },
}

impl Display for StreamError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidEvent => formatter.write_str("event type and payload are required"),
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
        let oldest = self
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or(sequence + 1);
        if !self.events.is_empty() && sequence.saturating_add(1) < oldest {
            return Err(StreamError::ResumeOutOfRange {
                requested: sequence,
                oldest,
            });
        }
        Ok(self
            .events
            .iter()
            .filter(|event| event.sequence > sequence)
            .cloned()
            .collect())
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

#[cfg(test)]
mod tests {
    use super::{EventStream, StreamError};

    #[test]
    fn preserves_order_resume_and_backpressure() {
        let mut stream = EventStream::new(2).expect("stream");
        let first = stream.publish("session.ready.v1", "{}").expect("event");
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
}
