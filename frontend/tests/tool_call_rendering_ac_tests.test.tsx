import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { ToolCall } from '@/components/ToolCall/ToolCall';
import {
  escapeHtml,
  redactArguments,
  type ToolCallData,
  type ToolCallState,
} from '@/components/ToolCall/types';

const baseCall = (state: ToolCallState): ToolCallData => ({
  id: 'call_1',
  name: 'filesystem_read',
  arguments: { path: 'notes.txt', api_key: 'secret-value' },
  state,
  projectId: 'project_1',
  agentId: 'agent_1',
  toolVersion: '1.0.0',
  traceId: 'trace_12345678',
  approvalId: 'approval_1',
});

describe('ToolCall', () => {
  it('renders every lifecycle state with its user-facing label @spec:AC-662', () => {
    const states: ToolCallState[] = [
      'pending', 'allowed', 'ask', 'denied', 'running',
      'succeeded', 'failed', 'cancelled', 'timeout',
    ];

    for (const state of states) {
      const { unmount } = render(<ToolCall data={baseCall(state)} />);
      expect(screen.getByText(stateLabel(state))).toBeInTheDocument();
      unmount();
    }
  });

  it('redacts secret-like argument keys and approval is only exposed for ask @spec:AC-663', () => {
    const onApprove = vi.fn().mockResolvedValue(undefined);
    const onDeny = vi.fn().mockResolvedValue(undefined);
    const { rerender } = render(
      <ToolCall data={baseCall('ask')} onApprove={onApprove} onDeny={onDeny} />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Mostrar (2 chaves)' }));
    expect(screen.getByText(/\[redigido\]/)).toBeInTheDocument();
    expect(screen.queryByText('secret-value')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Aprovar' }));
    expect(onApprove).toHaveBeenCalledWith('approval_1');

    rerender(<ToolCall data={baseCall('denied')} onApprove={onApprove} onDeny={onDeny} />);
    expect(screen.queryByRole('button', { name: 'Aprovar' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Negar' })).not.toBeInTheDocument();
  });

  it('renders hostile output as text and keeps result errors bounded to the DOM text @spec:AC-664', () => {
    const data = {
      ...baseCall('failed'),
      result: {
        success: false,
        output: { message: '<script>alert("xss")</script>' },
        error: '<img src=x onerror=alert(1)>',
      },
    };
    render(<ToolCall data={data} />);
    fireEvent.click(screen.getByText('Resultado ✗'));
    fireEvent.click(screen.getByRole('button', { name: 'Mostrar output' }));

    expect(document.querySelector('script')).toBeNull();
    expect(document.querySelector('img')).toBeNull();
    expect(screen.getByText(/<script>alert/)).toBeInTheDocument();
    expect(screen.getByText(/<img src=x/)).toBeInTheDocument();
  });

  it('redacts nested values, escapes standalone display helpers, and preserves project metadata @spec:AC-665', () => {
    const redacted = redactArguments({
      nested: { password: 'do-not-show' },
      safe: '<text>',
    });
    expect(redacted).toEqual({ nested: { password: '[redigido]' }, safe: '<text>' });
    expect(escapeHtml('<script>"x"</script>')).toBe('&lt;script&gt;&quot;x&quot;&lt;/script&gt;');

    render(<ToolCall data={baseCall('succeeded')} />);
    expect(screen.getByTitle('Trace: trace_12345678')).toBeInTheDocument();
  });
});

function stateLabel(state: ToolCallState): string {
  return {
    pending: 'Pendente',
    allowed: 'Permitido',
    ask: 'Aguardando aprovação',
    denied: 'Negado',
    running: 'Executando',
    succeeded: 'Concluído',
    failed: 'Falhou',
    cancelled: 'Cancelado',
    timeout: 'Tempo esgotado',
  }[state];
}
