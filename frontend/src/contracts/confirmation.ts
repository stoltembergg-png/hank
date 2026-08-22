export const CONFIRMATION_SCHEMA_VERSION = 1 as const;
export const CONFIRMATION_EVENT_NAME = 'hank://confirmation' as const;
const MAX_ID_CHARS = 128;

export type ConfirmationEffect =
  | 'read'
  | 'write'
  | 'execute'
  | 'network'
  | 'credentials'
  | 'payment'
  | 'install_package'
  | 'force_push';

export type ConfirmationPolicy =
  | 'always_allow'
  | 'ask_once'
  | 'ask_every_time'
  | 'deny';

/**
 * Bounded approval artifact: only hashes and identifiers cross the bridge,
 * never the raw schema or tool arguments.
 */
export type ConfirmationRequest = {
  request_id: string;
  project_id: string;
  agent_id: string | null;
  tool_name: string;
  tool_version: string;
  schema_hash: string;
  args_hash: string;
  effect: ConfirmationEffect;
  budget_ref: string | null;
  trace_id: string;
  actor_id: string;
  policy: ConfirmationPolicy;
  created_at_ms: number;
  expires_at_ms: number;
};

export type ConfirmationEventPayload = {
  kind: 'request_submitted';
  request: ConfirmationRequest;
};

export type ConfirmationEvent = {
  schema_version: number;
  event_id: string;
  request_id: string;
  sequence: number;
  payload: ConfirmationEventPayload;
};

const CONFIRMATION_EFFECTS: readonly ConfirmationEffect[] = [
  'read',
  'write',
  'execute',
  'network',
  'credentials',
  'payment',
  'install_package',
  'force_push',
];

const CONFIRMATION_POLICIES: readonly ConfirmationPolicy[] = [
  'always_allow',
  'ask_once',
  'ask_every_time',
  'deny',
];

const REQUEST_KEYS = [
  'request_id',
  'project_id',
  'agent_id',
  'tool_name',
  'tool_version',
  'schema_hash',
  'args_hash',
  'effect',
  'budget_ref',
  'trace_id',
  'actor_id',
  'policy',
  'created_at_ms',
  'expires_at_ms',
] as const;

/**
 * Accepts only current-schema confirmation events whose request carries the
 * exact bounded artifact fields; each extra key (for example a raw payload
 * field) invalidates the event.
 */
export function isConfirmationEvent(value: unknown): value is ConfirmationEvent {
  if (!isRecord(value)) return false;
  if (value.schema_version !== CONFIRMATION_SCHEMA_VERSION) return false;
  if (!validId(value.event_id) || !validId(value.request_id)) return false;
  if (!isSequence(value.sequence)) return false;
  const payload = value.payload;
  if (!isRecord(payload) || payload.kind !== 'request_submitted') return false;
  if (Object.keys(payload).length !== 2) return false;
  return isConfirmationRequest(payload.request);
}

export function isConfirmationRequest(value: unknown): value is ConfirmationRequest {
  if (!isRecord(value)) return false;
  const keys = Object.keys(value);
  if (keys.length !== REQUEST_KEYS.length) return false;
  for (const key of REQUEST_KEYS) {
    if (!(key in value)) return false;
  }
  if (!validId(value.request_id) || !validId(value.project_id)) return false;
  if (value.agent_id !== null && !validId(value.agent_id)) return false;
  if (!validId(value.tool_name) || !validId(value.tool_version)) return false;
  if (!validId(value.schema_hash) || !validId(value.args_hash)) return false;
  if (!CONFIRMATION_EFFECTS.includes(value.effect as ConfirmationEffect)) return false;
  if (value.budget_ref !== null && !validId(value.budget_ref)) return false;
  if (!validId(value.trace_id) || !validId(value.actor_id)) return false;
  if (!CONFIRMATION_POLICIES.includes(value.policy as ConfirmationPolicy)) return false;
  if (!isTimestamp(value.created_at_ms) || !isTimestamp(value.expires_at_ms)) return false;
  if (value.expires_at_ms < value.created_at_ms) return false;
  return true;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function validId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_ID_CHARS && !hasControl(value);
}

function isSequence(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0;
}

function isTimestamp(value: unknown): value is number {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0;
}

function hasControl(value: string): boolean {
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (code <= 0x1f || code === 0x7f) return true;
  }
  return false;
}
