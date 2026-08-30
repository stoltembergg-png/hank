import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { ProjectDetailView } from '@/components/ProjectDetailView';
import type { AgentApiClient } from '@/api/agents';
import type { AgentSummary } from '@/types/agent';
import type { ProjectSummary } from '@/types/project';

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

function createAgentApi(): AgentApiClient {
  return {
    list: vi.fn().mockResolvedValue({ agents: [agent], total: 1, limit: 10, offset: 0 }),
    get: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    archive: vi.fn(),
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
});
