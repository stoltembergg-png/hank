export const CHAT_STREAM_SCHEMA_VERSION = 1 as const;
export const CHAT_STREAM_EVENT_NAME = 'hank://chat/stream' as const;
const MAX_DELTA_BYTES = 65_536;

export type ChatStreamCaller = {
  caller_id: string;
  class: string;
};

export type ChatStreamSubscription = {
  stream_id: string;
  command_id: string;
  caller: ChatStreamCaller;
  project_id: string;
  agent_id: string;
  session_id: string;
  generation: number;
};

export type ChatStreamPayload =
  | { kind: 'start' }
  | { kind: 'delta'; text: string }
  | { kind: 'usage'; input_tokens: number; output_tokens: number }
  | { kind: 'finish'; reason: 'completed' | 'length' }
  | {
      kind: 'error';
      code: 'unauthorized' | 'provider_failure' | 'budget_exceeded' | 'invalid_stream' | 'backpressure' | 'unknown';
    }
  | { kind: 'cancel'; reason: 'user' | 'session_closed' | 'deadline' };

export type ChatStreamEvent = ChatStreamSubscription & {
  schema_version: number;
  sequence: number;
  payload: ChatStreamPayload;
};

export type ChatStreamRejectionReason =
  | 'invalid_event'
  | 'foreign_stream'
  | 'stale_generation'
  | 'future_generation'
  | 'must_start'
  | 'already_started'
  | 'duplicate_sequence'
  | 'out_of_order'
  | 'after_terminal';

export type ChatStreamResult =
  | { accepted: true; event: ChatStreamEvent }
  | {
      accepted: false;
      reason: ChatStreamRejectionReason;
      expected_sequence?: number;
    };

export class ChatStreamConsumer {
  private expectedSequence = 0;
  private started = false;
  private terminal = false;

  public constructor(private readonly subscription: ChatStreamSubscription) {}

  public accept(value: unknown): ChatStreamResult {
    if (!isChatStreamEvent(value)) {
      return { accepted: false, reason: 'invalid_event' };
    }
    const event = value;
    if (!sameIdentity(this.subscription, event)) {
      return { accepted: false, reason: 'foreign_stream' };
    }
    if (event.generation < this.subscription.generation) {
      return { accepted: false, reason: 'stale_generation' };
    }
    if (event.generation > this.subscription.generation) {
      return { accepted: false, reason: 'future_generation' };
    }
    if (this.terminal) {
      return { accepted: false, reason: 'after_terminal' };
    }
    if (event.sequence < this.expectedSequence) {
      return { accepted: false, reason: 'duplicate_sequence' };
    }
    if (event.sequence > this.expectedSequence) {
      return {
        accepted: false,
        reason: 'out_of_order',
        expected_sequence: this.expectedSequence,
      };
    }
    if (!this.started && event.payload.kind !== 'start') {
      return { accepted: false, reason: 'must_start' };
    }
    if (this.started && event.payload.kind === 'start') {
      return { accepted: false, reason: 'already_started' };
    }
    this.started = true;
    this.terminal = isTerminal(event.payload);
    this.expectedSequence += 1;
    return { accepted: true, event };
  }

  public isTerminal(): boolean {
    return this.terminal;
  }

  public nextSequence(): number {
    return this.expectedSequence;
  }
}

function sameIdentity(
  subscription: ChatStreamSubscription,
  event: ChatStreamEvent,
): boolean {
  return (
    subscription.stream_id === event.stream_id &&
    subscription.command_id === event.command_id &&
    subscription.caller.caller_id === event.caller.caller_id &&
    subscription.caller.class === event.caller.class &&
    subscription.project_id === event.project_id &&
    subscription.agent_id === event.agent_id &&
    subscription.session_id === event.session_id
  );
}

function isTerminal(payload: ChatStreamPayload): boolean {
  return payload.kind === 'finish' || payload.kind === 'error' || payload.kind === 'cancel';
}

function isChatStreamEvent(value: unknown): value is ChatStreamEvent {
  if (!isRecord(value)) return false;
  if (value.schema_version !== CHAT_STREAM_SCHEMA_VERSION) return false;
  if (!validId(value.stream_id) || !validId(value.command_id)) return false;
  if (!validId(value.project_id) || !validId(value.agent_id) || !validId(value.session_id)) return false;
  if (typeof value.generation !== 'number' || !Number.isInteger(value.generation) || value.generation <= 0) return false;
  if (typeof value.sequence !== 'number' || !Number.isInteger(value.sequence) || value.sequence < 0) return false;
  if (!isRecord(value.caller) || !validId(value.caller.caller_id) || !validId(value.caller.class)) {
    return false;
  }
  return isPayload(value.payload);
}

function isPayload(value: unknown): value is ChatStreamPayload {
  if (!isRecord(value) || typeof value.kind !== 'string') return false;
  switch (value.kind) {
    case 'start':
      return Object.keys(value).length === 1;
    case 'delta':
      return (
        typeof value.text === 'string' &&
        value.text.length > 0 &&
        new TextEncoder().encode(value.text).length <= MAX_DELTA_BYTES &&
        !value.text.includes('\0')
      );
    case 'usage':
      return (
        typeof value.input_tokens === 'number' &&
        typeof value.output_tokens === 'number' &&
        Number.isInteger(value.input_tokens) &&
        Number.isInteger(value.output_tokens) &&
        value.input_tokens >= 0 &&
        value.output_tokens >= 0 &&
        value.input_tokens + value.output_tokens > 0
      );
    case 'finish':
      return value.reason === 'completed' || value.reason === 'length';
    case 'error':
      return [
        'unauthorized',
        'provider_failure',
        'budget_exceeded',
        'invalid_stream',
        'backpressure',
        'unknown',
      ].includes(value.code as string);
    case 'cancel':
      return ['user', 'session_closed', 'deadline'].includes(value.reason as string);
    default:
      return false;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function validId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 128 && !hasControl(value);
}

function hasControl(value: string): boolean {
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (code <= 0x1f || code === 0x7f) return true;
  }
  return false;
}
