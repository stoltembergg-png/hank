import { useState, type CSSProperties } from 'react';
import { ToolCallProps, redactArguments, formatJsonForDisplay, TOOL_CALL_STATE_LABELS, TOOL_CALL_STATE_COLORS } from './types';
import './ToolCall.css';

const stateStyle = (color: string): CSSProperties => ({ '--tool-call-color': color } as CSSProperties);

export function ToolCall({ data, onApprove, onDeny, compact = false }: ToolCallProps) {
  const [showArgs, setShowArgs] = useState(false);
  const [showResult, setShowResult] = useState(false);

  const stateColor = TOOL_CALL_STATE_COLORS[data.state];
  const stateLabel = TOOL_CALL_STATE_LABELS[data.state];
  const isTerminal = ['succeeded', 'failed', 'cancelled', 'timeout'].includes(data.state);
  const isInteractive = ['ask'].includes(data.state);

  const redactedArgs = redactArguments(data.arguments);
  const argsJson = formatJsonForDisplay(redactedArgs);
  const resultOutput = data.result?.output as Record<string, unknown> | undefined;
  const resultJson = resultOutput ? formatJsonForDisplay(redactArguments(resultOutput)) : null;
  const errorText = data.result?.error ?? null;

  if (compact) {
    return (
      <div
        className="tool-call-compact"
        style={stateStyle(stateColor)}
        title={`${data.name} — ${stateLabel}`}
      >
        <span className="tool-call-badge" style={{ backgroundColor: stateColor }}>
          {data.name}
        </span>
        <span className="tool-call-state-badge">{stateLabel}</span>
      </div>
    );
  }

  return (
    <details className="tool-call" open={!isTerminal} data-state={data.state}>
      <summary className="tool-call-summary">
        <div className="tool-call-header">
          <span
            className="tool-call-badge"
            style={{ backgroundColor: stateColor }}
          >
            {data.name}
          </span>
          <span
            className="tool-call-state-badge"
            style={{ backgroundColor: stateColor }}
          >
            {stateLabel}
          </span>
          {data.traceId && (
            <span className="tool-call-trace" title={`Trace: ${data.traceId}`}>
              #{data.traceId.slice(0, 8)}
            </span>
          )}
        </div>
        {isInteractive && onApprove && onDeny && (
          <div className="tool-call-approval">
            <button
              className="tool-call-btn tool-call-btn-approve"
              onClick={() => onApprove?.(data.approvalId || '')}
              disabled={!data.approvalId}
            >
              Aprovar
            </button>
            <button
              className="tool-call-btn tool-call-btn-deny"
              onClick={() => onDeny?.(data.approvalId || '')}
              disabled={!data.approvalId}
            >
              Negar
            </button>
          </div>
        )}
      </summary>

      <div className="tool-call-body">
        <div className="tool-call-section">
          <h4>Argumentos</h4>
          <button
            className="tool-call-toggle"
            onClick={() => setShowArgs(!showArgs)}
            aria-expanded={showArgs}
          >
            {showArgs ? 'Ocultar' : 'Mostrar'} ({Object.keys(data.arguments).length} chaves)
          </button>
          {showArgs && (
            <pre className="tool-call-pre">{argsJson}</pre>
          )}
        </div>

        {data.result && (
          <div className="tool-call-section">
            <h4>Resultado {data.result.success ? '✓' : '✗'}</h4>
            <button
              className="tool-call-toggle"
              onClick={() => setShowResult(!showResult)}
              aria-expanded={showResult}
            >
              {showResult ? 'Ocultar' : 'Mostrar'} output
            </button>
            {showResult && (
              <>
                {resultJson && (
                  <pre className="tool-call-pre">{resultJson}</pre>
                )}
                {errorText && (
                  <div className="tool-call-error">
                    <strong>Erro:</strong> {errorText}
                  </div>
                )}
              </>
            )}
          </div>
        )}

        {data.budget && (
          <div className="tool-call-section tool-call-budget">
            <h4>Orçamento</h4>
            <div className="tool-call-budget-grid">
              <span>Tokens: {data.budget.tokensUsed.toLocaleString()}</span>
              <span>Custo: ${(data.budget.costMicros / 1_000_000).toFixed(4)}</span>
            </div>
          </div>
        )}

        {data.startedAt && (
          <div className="tool-call-section tool-call-timing">
            <h4>Tempo</h4>
            <div className="tool-call-timing-grid">
              <span>Início: {new Date(data.startedAt).toLocaleTimeString()}</span>
              {data.completedAt && (
                <span>Fim: {new Date(data.completedAt).toLocaleTimeString()}</span>
              )}
            </div>
          </div>
        )}
      </div>
    </details>
  );
}