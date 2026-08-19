use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::broadcast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventBusError {
    Closed,
    NoSubscribers,
    Lagged(u64),
}

#[derive(Debug)]
pub struct EventBus<T: Clone> {
    sender: broadcast::Sender<T>,
    closed: Arc<AtomicBool>,
}

impl<T: Clone> Clone for EventBus<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            closed: Arc::clone(&self.closed),
        }
    }
}

impl<T: Clone> EventBus<T> {
    pub fn bounded(capacity: usize) -> Self {
        assert!(capacity > 0, "event bus capacity must be positive");
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: T) -> Result<usize, EventBusError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(EventBusError::Closed);
        }
        self.sender
            .send(event)
            .map_err(|_| EventBusError::NoSubscribers)
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

pub fn receive<T: Clone>(receiver: &mut broadcast::Receiver<T>) -> Result<T, EventBusError> {
    receiver.try_recv().map_err(|error| match error {
        broadcast::error::TryRecvError::Empty => EventBusError::NoSubscribers,
        broadcast::error::TryRecvError::Closed => EventBusError::Closed,
        broadcast::error::TryRecvError::Lagged(count) => EventBusError::Lagged(count),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publishes_in_order_and_closes_new_publication() {
        let bus = EventBus::bounded(4);
        let mut receiver = bus.subscribe();
        assert_eq!(bus.publish(1).unwrap(), 1);
        assert_eq!(bus.publish(2).unwrap(), 1);
        assert_eq!(receiver.recv().await.unwrap(), 1);
        assert_eq!(receiver.recv().await.unwrap(), 2);
        bus.close();
        assert_eq!(bus.publish(3), Err(EventBusError::Closed));
    }

    #[tokio::test]
    async fn bounded_bus_reports_lag_without_unbounded_memory() {
        let bus = EventBus::bounded(1);
        let mut receiver = bus.subscribe();
        bus.publish(1).unwrap();
        bus.publish(2).unwrap();
        assert_eq!(receive(&mut receiver), Err(EventBusError::Lagged(1)));
        assert_eq!(receive(&mut receiver).unwrap(), 2);
    }
}
