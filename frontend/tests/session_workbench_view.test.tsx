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
  it('exibe metadados da sessão e mantém o envio bloqueado sem bridge de execução', () => {
    const onBack = vi.fn();

    render(
      <SessionWorkbench
        session={session}
        agentName="release-agent"
        onBack={onBack}
      />,
    );

    expect(screen.getByRole('heading', { name: 'Preparar release Windows' })).toBeInTheDocument();
    expect(screen.getByText('Conversa com release-agent')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Resumo da sessão' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Área da conversa' })).toBeInTheDocument();
    expect(document.querySelector('.session-workbench-agent-avatar')).not.toBeNull();
    expect(screen.getByText('2 mensagens registradas')).toBeInTheDocument();
    expect(screen.getByText('Envio de mensagens ainda não está integrado ao desktop.')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Mensagem' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Enviar mensagem' })).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: 'Voltar para conversas' }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
