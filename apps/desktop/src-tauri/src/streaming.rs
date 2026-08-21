use agent_protocol::chat_stream::{
    ChatStreamEvent, ChatStreamQueue, ChatStreamSubscription, ChatStreamValidationError,
    CHAT_STREAM_EVENT_NAME,
};
use tauri::{Emitter, WebviewWindow};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StreamSinkError {
    #[error("stream event sink is unavailable")]
    Unavailable,
}

pub trait StreamEventSink {
    fn emit(&mut self, event_name: &str, event: &ChatStreamEvent) -> Result<(), StreamSinkError>;
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum StreamBridgeError {
    #[error("stream validation rejected event: {0}")]
    Validation(#[from] ChatStreamValidationError),
    #[error("stream event sink failed: {0}")]
    Sink(#[from] StreamSinkError),
}

pub struct StreamBridge<S> {
    validator: agent_protocol::chat_stream::ChatStreamValidator,
    queue: ChatStreamQueue,
    sink: S,
}

impl<S: StreamEventSink> StreamBridge<S> {
    pub fn new(
        subscription: ChatStreamSubscription,
        max_events: usize,
        sink: S,
    ) -> Result<Self, ChatStreamValidationError> {
        Ok(Self {
            validator: agent_protocol::chat_stream::ChatStreamValidator::new(subscription)?,
            queue: ChatStreamQueue::new(max_events)?,
            sink,
        })
    }

    pub fn publish(&mut self, event: ChatStreamEvent) -> Result<(), StreamBridgeError> {
        let mut validator = self.validator.clone();
        validator.accept(event.clone())?;
        let mut queue = self.queue.clone();
        queue.push(event)?;
        self.validator = validator;
        self.queue = queue;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<usize, StreamBridgeError> {
        let mut emitted = 0;
        while let Some(event) = self.queue.front().cloned() {
            self.sink.emit(CHAT_STREAM_EVENT_NAME, &event)?;
            let _ = self.queue.pop();
            emitted += 1;
        }
        Ok(emitted)
    }

    pub fn queued_len(&self) -> usize {
        self.queue.len()
    }

    pub fn coalesced_count(&self) -> u64 {
        self.queue.coalesced_count()
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    pub fn into_sink(self) -> S {
        self.sink
    }
}

pub struct TauriWindowSink {
    window: WebviewWindow,
}

impl TauriWindowSink {
    pub fn new(window: WebviewWindow) -> Self {
        Self { window }
    }
}

impl StreamEventSink for TauriWindowSink {
    fn emit(&mut self, event_name: &str, event: &ChatStreamEvent) -> Result<(), StreamSinkError> {
        self.window
            .emit(event_name, event)
            .map_err(|_| StreamSinkError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_protocol::chat_command::CallerIdentity;
    use agent_protocol::chat_stream::{
        ChatStreamEvent, ChatStreamPayload, ChatStreamSubscription, ChatTerminalReason,
        CHAT_STREAM_EVENT_NAME,
    };
    use agent_protocol::ids::{AgentId, ProjectId, SessionId};

    #[derive(Default)]
    struct FakeSink {
        events: Vec<ChatStreamEvent>,
        fail: bool,
    }

    impl StreamEventSink for FakeSink {
        fn emit(
            &mut self,
            event_name: &str,
            event: &ChatStreamEvent,
        ) -> Result<(), StreamSinkError> {
            assert_eq!(event_name, CHAT_STREAM_EVENT_NAME);
            if self.fail {
                return Err(StreamSinkError::Unavailable);
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    fn subscription() -> ChatStreamSubscription {
        ChatStreamSubscription::new(
            "stream-1",
            "command-1",
            CallerIdentity::new("caller-1", "desktop").unwrap(),
            ProjectId::new(),
            AgentId::new(),
            SessionId::new(),
            1,
        )
        .unwrap()
    }

    fn event(
        subscription: &ChatStreamSubscription,
        sequence: u64,
        payload: ChatStreamPayload,
    ) -> ChatStreamEvent {
        ChatStreamEvent::new(subscription, sequence, payload).unwrap()
    }

    #[test]
    fn bridge_flushes_ordered_events_to_named_channel() {
        let subscription = subscription();
        let mut bridge = StreamBridge::new(subscription.clone(), 4, FakeSink::default()).unwrap();
        bridge
            .publish(event(&subscription, 0, ChatStreamPayload::Start))
            .unwrap();
        bridge
            .publish(event(
                &subscription,
                1,
                ChatStreamPayload::Delta {
                    text: "hello".into(),
                },
            ))
            .unwrap();
        bridge
            .publish(event(
                &subscription,
                2,
                ChatStreamPayload::Finish {
                    reason: ChatTerminalReason::Completed,
                },
            ))
            .unwrap();
        assert_eq!(bridge.flush().unwrap(), 3);
        let sink = bridge.into_sink();
        assert_eq!(sink.events.len(), 3);
        assert_eq!(sink.events[0].sequence, 0);
        assert!(sink.events[2].is_terminal());
    }

    #[test]
    fn bridge_rejects_foreign_and_stale_events_before_queueing() {
        let subscription = subscription();
        let mut bridge = StreamBridge::new(subscription.clone(), 4, FakeSink::default()).unwrap();
        bridge
            .publish(event(&subscription, 0, ChatStreamPayload::Start))
            .unwrap();
        let mut foreign = event(
            &subscription,
            1,
            ChatStreamPayload::Delta { text: "x".into() },
        );
        foreign.caller = CallerIdentity::new("other", "desktop").unwrap();
        assert!(matches!(
            bridge.publish(foreign),
            Err(StreamBridgeError::Validation(_))
        ));
        let mut stale = event(
            &subscription,
            1,
            ChatStreamPayload::Delta {
                text: "stale".into(),
            },
        );
        stale.generation = 0;
        assert!(matches!(
            bridge.publish(stale),
            Err(StreamBridgeError::Validation(_))
        ));
        assert_eq!(bridge.queued_len(), 1);
    }

    #[test]
    fn bridge_applies_bounded_backpressure_and_preserves_terminal_event() {
        let subscription = subscription();
        let mut bridge = StreamBridge::new(subscription.clone(), 2, FakeSink::default()).unwrap();
        bridge
            .publish(event(&subscription, 0, ChatStreamPayload::Start))
            .unwrap();
        bridge
            .publish(event(
                &subscription,
                1,
                ChatStreamPayload::Delta { text: "one".into() },
            ))
            .unwrap();
        assert!(matches!(
            bridge.publish(event(
                &subscription,
                2,
                ChatStreamPayload::Delta { text: "two".into() },
            )),
            Err(StreamBridgeError::Validation(_))
        ));
        bridge
            .publish(event(
                &subscription,
                2,
                ChatStreamPayload::Finish {
                    reason: ChatTerminalReason::Completed,
                },
            ))
            .unwrap();
        assert_eq!(bridge.coalesced_count(), 1);
        bridge.flush().unwrap();
        let sink = bridge.into_sink();
        assert!(sink.events[0].is_start());
        assert!(sink.events[1].is_terminal());
    }

    #[test]
    fn sink_failure_does_not_drop_queued_event() {
        let subscription = subscription();
        let sink = FakeSink {
            fail: true,
            ..FakeSink::default()
        };
        let mut bridge = StreamBridge::new(subscription.clone(), 2, sink).unwrap();
        bridge
            .publish(event(&subscription, 0, ChatStreamPayload::Start))
            .unwrap();
        assert_eq!(
            bridge.flush(),
            Err(StreamBridgeError::Sink(StreamSinkError::Unavailable))
        );
        assert_eq!(bridge.queued_len(), 1);
        bridge.sink_mut().fail = false;
        assert_eq!(bridge.flush().unwrap(), 1);
    }
}
