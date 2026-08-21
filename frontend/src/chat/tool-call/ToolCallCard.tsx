import './ToolCallCard.css';

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

export type ToolCallViewModel = {
  id: string;
  projectId: string;
  agentId: string;
  toolName: string;
  toolVersion: string;
  traceId: string;
  state: ToolCallState;
  arguments?: unknown;
  result?: unknown;
  error?: string;
};

type ToolCallCardProps = {
  call: ToolCallViewModel;
  onApprove?: () => void;
};

const MAX_PAYLOAD_CHARS = 4_000;
const MAX_OBJECT_ENTRIES = 50;
const MAX_NESTING_DEPTH = 4;
const SENSITIVE_KEY = /(?:token|secret|password|authorization|api[_-]?key|credential)/i;
const SENSITIVE_VALUE = /^(?:sk-|gh[pousr]_\w+|AKIA\w+|-----BEGIN)/i;

const STATE_LABELS: Record<ToolCallState, string> = {
  pending: 'Pendente',
  allowed: 'Autorizada',
  ask: 'Aprovação necessária',
  denied: 'Negada',
  running: 'Executando',
  succeeded: 'Concluída',
  failed: 'Falhou',
  cancelled: 'Cancelada',
  timeout: 'Tempo esgotado',
};

export function ToolCallCard({ call, onApprove }: ToolCallCardProps) {
  const argumentsText = formatPayload(call.arguments);
  const resultText = formatPayload(call.result);
  const statusLabel = STATE_LABELS[call.state];

  return (
    <article className={`tool-call-card tool-call-card-${call.state}`} aria-label={`Tool call ${call.toolName}`}>
      <header className="tool-call-card__header">
        <div>
          <p className="tool-call-card__eyebrow">Tool call</p>
          <h2>{call.toolName}</h2>
          <p className="tool-call-card__version">Versão {call.toolVersion}</p>
        </div>
        <span className="tool-call-card__status" role="status">{statusLabel}</span>
      </header>

      <dl className="tool-call-card__metadata">
        <div>
          <dt>Escopo</dt>
          <dd>Projeto {call.projectId} · Agente {call.agentId}</dd>
        </div>
        <div>
          <dt>Rastreamento</dt>
          <dd>Trace {call.traceId}</dd>
        </div>
      </dl>

      {argumentsText && <PayloadBlock label="Argumentos" value={argumentsText} />}
      {resultText && <PayloadBlock label="Resultado" value={resultText} />}
      {call.error && <PayloadBlock label="Erro" value={call.error} />}

      {call.state === 'ask' && (
        <div className="tool-call-card__approval" role="group" aria-label="Aprovação da ferramenta">
          <p>A aprovação será processada pela Application API.</p>
          <button type="button" onClick={onApprove} disabled={!onApprove}>Solicitar aprovação</button>
        </div>
      )}

      {call.state === 'denied' && (
        <p className="tool-call-card__notice">A ferramenta não pode ser executada a partir deste estado.</p>
      )}
    </article>
  );
}

function PayloadBlock({ label, value }: { label: string; value: string }) {
  return (
    <section className="tool-call-card__payload" aria-label={label}>
      <h3>{label}</h3>
      <pre>{value}</pre>
    </section>
  );
}

function formatPayload(value: unknown): string | null {
  if (value === undefined) return null;

  const sanitized = sanitizePayload(value, 0);
  const serialized = typeof sanitized === 'string'
    ? sanitized
    : JSON.stringify(sanitized, null, 2);
  if (serialized.length <= MAX_PAYLOAD_CHARS) return serialized;
  return `${serialized.slice(0, MAX_PAYLOAD_CHARS)}\n… Conteúdo truncado`;
}

function sanitizePayload(value: unknown, depth: number): unknown {
  if (depth >= MAX_NESTING_DEPTH) return '[conteúdo aninhado omitido]';
  if (typeof value === 'string') return SENSITIVE_VALUE.test(value) ? '[redigido]' : value;
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.slice(0, MAX_OBJECT_ENTRIES).map((item) => sanitizePayload(item, depth + 1));

  return Object.fromEntries(
    Object.entries(value).slice(0, MAX_OBJECT_ENTRIES).map(([key, item]) => [
      key,
      SENSITIVE_KEY.test(key) ? '[redigido]' : sanitizePayload(item, depth + 1),
    ]),
  );
}
