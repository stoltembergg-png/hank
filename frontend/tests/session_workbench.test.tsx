import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AgentList } from '@/components/AgentList';
import type { AgentApiClient } from '@/api/agents';
import type { SessionApiClient } from '@/api/sessions';
import type { AgentSummary } from '@/types/agent';
import type { SessionSummary } from '@/types/session';

const projectId = 'proj-00000000-0000-4000-8000-000000000001';
const agent: AgentSummary = {
  id: 'agent-00000000-0000-4000-8000-000000000002',
  project_id: projectId,
  name: 'release-agent',
  description: 'Prepara releases com revisão humana.',
  status: 'active',
  personality: {
    name: 'Default',
    description: null,
    traits: ['helpful'],
    communication_style: 'technical',
  },
  created_at: '2026-08-30T08:00:00.000Z',
  updated_at: '2026-08-30T08:00:00.000Z',
};

const session: SessionSummary = {
  id: 'session-00000000-0000-4000-8000-000000000003',
  project_id: projectId,
  agent_id: agent.id,
  status: 'active',
  title: 'Preparar release Windows',
  message_count: 0,
  token_count: 0,
  created_at: '2026-08-30T08:05:00.000Z',
  updated_at: '2026-08-30T08:05:00.000Z',
  closed_at: null,
};

function agentApiClient(): AgentApiClient {
  return {
    list: vi.fn().mockResolvedValue({ agents: [agent], total: 1, limit: 10, offset: 0 }),
    get: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    archive: vi.fn(),
  };
}

function sessionApiClient(listResponse: SessionSummary[] = []): SessionApiClient {
  return {
    list: vi.fn().mockResolvedValue({
      sessions: listResponse,
      total: listResponse.length,
      limit: 10,
      offset: 0,
    }),
    create: vi.fn().mockResolvedValue({ session, correlation_id: 'corr-session' }),
  };
}

describe('Session workbench', () => {
  it('lists and creates sessions for the selected active agent', async () => {
    const agents = agentApiClient();
    const sessions = sessionApiClient();

    render(
      <AgentList
        projectId={projectId}
        apiClient={agents}
        sessionApiClient={sessions}
      />,
    );

    await screen.findByText('release-agent');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir conversas de release-agent' }));

    expect(await screen.findByText('Nenhuma conversa iniciada para este agent.')).toBeInTheDocument();
    expect(sessions.list).toHaveBeenCalledWith({
      project_id: projectId,
      agent_id: agent.id,
      limit: 10,
      offset: 0,
    });

    fireEvent.click(screen.getByRole('button', { name: 'Abrir formulário de nova conversa' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Título da conversa' }), {
      target: { value: 'Preparar release Windows' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Criar conversa' }));

    await waitFor(() => expect(sessions.create).toHaveBeenCalledWith({
      project_id: projectId,
      agent_id: agent.id,
      title: 'Preparar release Windows',
      correlation_id: expect.any(String),
    }));
  });

  it('does not expose conversations for inactive agents', async () => {
    const inactiveAgent = { ...agent, id: 'agent-inactive', name: 'inactive-agent', status: 'inactive' as const };
    const agents = agentApiClient();
    vi.mocked(agents.list).mockResolvedValue({ agents: [inactiveAgent], total: 1, limit: 10, offset: 0 });
    const sessions = sessionApiClient();

    render(
      <AgentList
        projectId={projectId}
        apiClient={agents}
        sessionApiClient={sessions}
      />,
    );

    await screen.findByText('inactive-agent');
    expect(screen.queryByRole('button', { name: 'Abrir conversas de inactive-agent' })).not.toBeInTheDocument();
    expect(sessions.list).not.toHaveBeenCalled();
  });
});
