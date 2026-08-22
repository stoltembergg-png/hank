// Tool call rendering types and utilities

export type ToolCallState =
  | 'pending'
  | 'allowed'
  | 'ask'
  | 'denied'
  | 'running'
  | 'succeeded'
  | 'failed'
  | 'cancelled'
  | 'timeout';

export interface ToolCallData {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  state: ToolCallState;
  projectId: string;
  agentId: string;
  toolVersion: string;
  traceId?: string;
  approvalId?: string;
  startedAt?: string;
  completedAt?: string;
  result?: {
    success: boolean;
    output: unknown;
    error?: string;
  };
  budget?: {
    tokensUsed: number;
    costMicros: number;
  };
}

export interface ToolCallProps {
  data: ToolCallData;
  onApprove?: (approvalId: string) => Promise<void>;
  onDeny?: (approvalId: string) => Promise<void>;
  compact?: boolean;
}

export const TOOL_CALL_STATE_LABELS: Record<ToolCallState, string> = {
  pending: 'Pendente',
  allowed: 'Permitido',
  ask: 'Aguardando aprovação',
  denied: 'Negado',
  running: 'Executando',
  succeeded: 'Concluído',
  failed: 'Falhou',
  cancelled: 'Cancelado',
  timeout: 'Tempo esgotado',
};

export const TOOL_CALL_STATE_COLORS: Record<ToolCallState, string> = {
  pending: 'var(--tool-call-pending, #f59e0b)',
  allowed: 'var(--tool-call-allowed, #10b981)',
  ask: 'var(--tool-call-ask, #3b82f6)',
  denied: 'var(--tool-call-denied, #ef4444)',
  running: 'var(--tool-call-running, #6366f1)',
  succeeded: 'var(--tool-call-succeeded, #22c55e)',
  failed: 'var(--tool-call-failed, #ef4444)',
  cancelled: 'var(--tool-call-cancelled, #6b7280)',
  timeout: 'var(--tool-call-timeout, #f97316)',
};

// Secret patterns for redaction
const SECRET_PATTERNS = [
  /api[_-]?key/i,
  /secret/i,
  /password/i,
  /token/i,
  /authorization/i,
  /bearer/i,
  /private[_-]?key/i,
  /access[_-]?token/i,
  /refresh[_-]?token/i,
  /client[_-]?secret/i,
];

function redactValue(value: unknown): unknown {
  if (typeof value === 'string') {
    return value.length > 1000 ? value.slice(0, 1000) + '… [truncado]' : value;
  }
  if (Array.isArray(value)) {
    return value.map(redactValue);
  }
  if (value && typeof value === 'object') {
    const redacted: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(value)) {
      const lowerKey = key.toLowerCase();
      if (SECRET_PATTERNS.some((pattern) => pattern.test(lowerKey))) {
        redacted[key] = '[redigido]';
      } else {
        redacted[key] = redactValue(val);
      }
    }
    return redacted;
  }
  return value;
}

export function redactArguments(args: Record<string, unknown>): Record<string, unknown> {
  return redactValue(args) as Record<string, unknown>;
}

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

export function formatJsonForDisplay(obj: unknown): string {
  try {
    return JSON.stringify(obj, null, 2);
  } catch {
    return '[não serializável]';
  }
}