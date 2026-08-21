import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ToolCallCard, type ToolCallViewModel } from '@/chat/tool-call/ToolCallCard';

const baseCall: ToolCallViewModel = {
  id: 'tool-call-1',
  projectId: 'proj-1',
  agentId: 'agent-1',
  toolName: 'git_commit',
  toolVersion: '1.0.0',
  traceId: 'trace-1',
  state: 'succeeded',
  arguments: { message: 'fix: safe rendering' },
  result: { commit: 'abc123' },
};

describe('ToolCallCard', () => {
  // @spec:AC-662
  it.each([
    ['pending', 'Pendente'],
    ['allowed', 'Autorizada'],
    ['ask', 'Aprovação necessária'],
    ['denied', 'Negada'],
    ['running', 'Executando'],
    ['succeeded', 'Concluída'],
    ['failed', 'Falhou'],
    ['cancelled', 'Cancelada'],
    ['timeout', 'Tempo esgotado'],
  ] as const)('renders the %s state as an accessible status', (state, label) => {
    render(<ToolCallCard call={{ ...baseCall, state }} />);

    expect(screen.getByRole('status')).toHaveTextContent(label);
    expect(screen.getByText('git_commit')).toBeInTheDocument();
    expect(screen.getByText('Projeto proj-1 · Agente agent-1')).toBeInTheDocument();
    expect(screen.getByText('Trace trace-1')).toBeInTheDocument();
  });

  // @spec:AC-663
  it('renders arguments and results as bounded text with secrets redacted', () => {
    render(
      <ToolCallCard
        call={{
          ...baseCall,
          arguments: {
            message: '<img src=x onerror=alert(1)>',
            token: 'secret-token',
            nested: { password: 'secret-password' },
          },
          result: 'x'.repeat(5000),
        }}
      />,
    );

    expect(screen.getByText(/<img src=x onerror=alert\(1\)>/)).toBeInTheDocument();
    expect(screen.queryByText('secret-token')).not.toBeInTheDocument();
    expect(screen.queryByText('secret-password')).not.toBeInTheDocument();
    expect(screen.getAllByText(/Conteúdo truncado/)).toHaveLength(1);
  });

  // @spec:AC-664
  it('exposes approval context without executing a tool locally', () => {
    const onApprove = vi.fn();
    render(<ToolCallCard call={{ ...baseCall, state: 'ask' }} onApprove={onApprove} />);

    fireEvent.click(screen.getByRole('button', { name: 'Solicitar aprovação' }));
    expect(onApprove).toHaveBeenCalledTimes(1);
    expect(screen.getByText('A aprovação será processada pela Application API.')).toBeInTheDocument();
  });

  // @spec:AC-664
  it('does not offer an approval action for denied calls', () => {
    render(<ToolCallCard call={{ ...baseCall, state: 'denied' }} onApprove={vi.fn()} />);

    expect(screen.queryByRole('button', { name: 'Solicitar aprovação' })).not.toBeInTheDocument();
    expect(screen.getByText('A ferramenta não pode ser executada a partir deste estado.')).toBeInTheDocument();
  });
});
