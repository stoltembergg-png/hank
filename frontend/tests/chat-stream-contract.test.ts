import { describe, expect, it } from 'vitest';
import {
  CHAT_STREAM_SCHEMA_VERSION,
  ChatStreamConsumer,
  type ChatStreamEvent,
  type ChatStreamSubscription,
} from '@/contracts/chat-stream';

const subscription: ChatStreamSubscription = {
  stream_id: 'stream-1',
  command_id: 'command-1',
  caller: { caller_id: 'caller-1', class: 'desktop' },
  project_id: 'proj-00000000-0000-4000-8000-000000000001',
  agent_id: 'agent-00000000-0000-4000-8000-000000000002',
  session_id: 'sess-00000000-0000-4000-8000-000000000003',
  generation: 1,
};

function event(
  sequence: number,
  payload: ChatStreamEvent['payload'],
): ChatStreamEvent {
  return {
    schema_version: CHAT_STREAM_SCHEMA_VERSION,
    ...subscription,
    generation: 1,
    sequence,
    payload,
  };
}

describe('ChatStreamConsumer', () => {
  it('accepts ordered events and exactly one terminal event', () => {
    const consumer = new ChatStreamConsumer(subscription);
    expect(consumer.accept(event(0, { kind: 'start' })).accepted).toBe(true);
    expect(consumer.accept(event(1, { kind: 'delta', text: 'hello' })).accepted).toBe(true);
    expect(consumer.accept(event(2, { kind: 'finish', reason: 'completed' })).accepted).toBe(true);
    expect(consumer.accept(event(3, { kind: 'delta', text: 'late' }))).toEqual({
      accepted: false,
      reason: 'after_terminal',
    });
  });

  it('rejects foreign, stale, duplicate and out-of-order events without mutation', () => {
    const consumer = new ChatStreamConsumer(subscription);
    expect(consumer.accept(event(0, { kind: 'start' })).accepted).toBe(true);
    const foreign = event(1, { kind: 'delta', text: 'foreign' });
    foreign.session_id = 'sess-00000000-0000-4000-8000-000000000099';
    expect(consumer.accept(foreign)).toEqual({ accepted: false, reason: 'foreign_stream' });
    const gap = event(2, { kind: 'delta', text: 'gap' });
    expect(consumer.accept(gap)).toEqual({
      accepted: false,
      reason: 'out_of_order',
      expected_sequence: 1,
    });
    expect(consumer.accept(event(1, { kind: 'delta', text: 'one' })).accepted).toBe(true);
    const duplicate = event(1, { kind: 'delta', text: 'duplicate' });
    expect(consumer.accept(duplicate)).toEqual({ accepted: false, reason: 'duplicate_sequence' });
    const stale = event(2, { kind: 'delta', text: 'stale' });
    stale.generation = 0;
    expect(consumer.accept(stale)).toEqual({ accepted: false, reason: 'invalid_event' });
  });

  it('rejects malformed payloads and oversized deltas without throwing', () => {
    const consumer = new ChatStreamConsumer(subscription);
    expect(consumer.accept({})).toEqual({ accepted: false, reason: 'invalid_event' });
    expect(
      consumer.accept(event(0, { kind: 'delta', text: 'x'.repeat(65_537) })),
    ).toEqual({ accepted: false, reason: 'invalid_event' });
    expect(consumer.accept(event(0, { kind: 'unknown' } as never))).toEqual({
      accepted: false,
      reason: 'invalid_event',
    });
  });
});
