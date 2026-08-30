import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ComponentProps } from 'react';
import { ProjectDetailView } from '@/components/ProjectDetailView';
import type { AgentApiClient } from '@/api/agents';
import type { SessionApiClient } from '@/api/sessions';
import type { AgentSummary } from '@/types/agent';
import type { ProjectSummary } from '@/types/project';
import type { SessionSummary } from '@/types/session';

const project: ProjectSummary = {
  id: 'prj_01j7x000000000000000000042',
  name: 'Workspace de Release',
  description: 'Projeto para validar a release',
  status: 'active',
  owner: 'gabriel',
  created_at: '2026-08-19T10:00:00.000Z',
  updated_at: '2026-08-19T12:00:00.000Z',
  settings: {
    retention_days: 30,
    auto_archive_idle_days: 14,
    telemetry_enabled: true,
    max_active_agents: 3,
  },
};

const agent: AgentSummary = {
  id: 'agt_01j7x000000000000000000007',
  project_id: project.id,
  name: 'release-agent',
  description: 'Valida a release do projeto',
  status: 'active',
  personality: {
    name: 'Default',
    description: null,
    traits: ['helpful', 'accurate'],
    communication_style: 'technical',
  },
  created_at: '2026-08-19T13:00:00.000Z',
  updated_at: '2026-08-19T13:00:00.000Z',
};

const session: SessionSummary = {
  id: 'ses_01j7x000000000000000000009',
  project_id: project.id,
  agent_id: agent.id,
  status: 'active',
  title: 'Validar a próxima release',
  message_count: 0,
  token_count: 0,
  created_at: '2026-08-19T13:05:00.000Z',
  updated_at: '2026-08-19T13:05:00.000Z',
  closed_at: null,
};

function createAgentApi(): AgentApiClient {
  return {
    list: vi.fn().mockResolvedValue({ agents: [agent], total: 1, limit: 10, offset: 0 }),
    get: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    archive: vi.fn(),
  };
}

function createSessionApi(): SessionApiClient {
  return {
    list: vi.fn().mockResolvedValue({
      sessions: [session],
      total: 1,
      limit: 10,
      offset: 0,
    }),
    create: vi.fn(),
  };
}

describe('Project Agents workbench', () => {
  it('opens the Agents tab and loads agents for the selected project', async () => {
    const agentApiClient = createAgentApi();

    render(
      <ProjectDetailView
        projectId={project.id}
        initialProject={project}
        agentApiClient={agentApiClient}
      />,
    );

    fireEvent.click(screen.getByRole('tab', { name: 'Agents' }));

    expect(await screen.findByText('release-agent')).toBeInTheDocument();
    await waitFor(() => {
      expect(agentApiClient.list).toHaveBeenCalledWith({
        project_id: project.id,
        limit: 10,
        offset: 0,
      });
    });
  });

  it('opens a selected session in the read-only session workbench', async () => {
    const agentApiClient = createAgentApi();
    const sessionApiClient = createSessionApi();
    const props = {
      projectId: project.id,
      initialProject: project,
      agentApiClient,
      sessionApiClient,
    } as ComponentProps<typeof ProjectDetailView>;

    render(<ProjectDetailView {...props} />);

    fireEvent.click(screen.getByRole('tab', { name: 'Agents' }));
    await screen.findByText('release-agent');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir conversas de release-agent' }));
    await screen.findByRole('button', { name: 'Abrir conversa' });

    fireEvent.click(screen.getByRole('button', { name: 'Abrir conversa' }));

    expect(await screen.findByRole('heading', { name: 'Validar a próxima release' })).toBeInTheDocument();
    expect(screen.getByText('Envio de mensagens ainda não está integrado ao desktop.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Voltar para conversas' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Voltar para conversas' }));
    expect(screen.getByRole('region', { name: 'Conversas de release-agent' })).toBeInTheDocument();
  });
});
