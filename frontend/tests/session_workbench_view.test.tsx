import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SessionWorkbench } from '@/components/SessionWorkbench';
import type { SessionSummary } from '@/types/session';

const session: SessionSummary = {
  id: 'session-00000000-0000-4000-8000-000000000003',
  project_id: 'proj-00000000-0000-4000-8000-000000000001',
  agent_id: 'agent-00000000-0000-4000-8000-000000000002',
  status: 'active',
  title: 'Preparar release Windows',
  message_count: 2,
  token_count: 128,
  created_at: '2026-08-30T08:05:00.000Z',
  updated_at: '2026-08-30T08:10:00.000Z',
  closed_at: null,
};

describe('Session workbench view', () => {
  it('exibe metadados da sessão e conecta o composer ao transporte de chat', () => {
    const onBack = vi.fn();
    const transport = {
      send: vi.fn().mockResolvedValue(undefined),
      cancel: vi.fn().mockResolvedValue(undefined),
      subscribe: vi.fn().mockReturnValue(() => undefined),
    };

    render(
      <SessionWorkbench
        session={session}
        agentName="release-agent"
        onBack={onBack}
        transport={transport}
      />,
    );

    expect(screen.getByRole('heading', { name: 'Preparar release Windows' })).toBeInTheDocument();
    expect(screen.getByText('Conversa com release-agent')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Resumo da sessão' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Área da conversa' })).toBeInTheDocument();
    expect(document.querySelector('.session-workbench-agent-avatar')).not.toBeNull();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Mensagem' })).not.toBeDisabled();
    expect(screen.getByRole('button', { name: 'Enviar mensagem' })).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: 'Voltar para conversas' }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
