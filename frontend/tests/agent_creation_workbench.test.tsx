import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AgentList } from '@/components/AgentList';
import type { AgentApiClient } from '@/api/agents';
import type { AgentSummary } from '@/types/agent';

const projectId = 'proj-00000000-0000-4000-8000-000000000001';

const createdAgent: AgentSummary = {
  id: 'agent-00000000-0000-4000-8000-000000000002',
  project_id: projectId,
  name: 'release-agent',
  description: 'Prepara releases com revisão humana.',
  status: 'active',
  personality: {
    name: 'Default',
    description: null,
    traits: ['helpful', 'accurate'],
    communication_style: 'technical',
  },
  created_at: '2026-08-30T08:00:00.000Z',
  updated_at: '2026-08-30T08:00:00.000Z',
};

describe('Agent creation workbench', () => {
  it('creates a project-scoped agent and refreshes the list', async () => {
    const apiClient: AgentApiClient = {
      list: vi.fn()
        .mockResolvedValueOnce({ agents: [], total: 0, limit: 10, offset: 0 })
        .mockResolvedValue({ agents: [createdAgent], total: 1, limit: 10, offset: 0 }),
      create: vi.fn().mockResolvedValue({ agent: createdAgent }),
      get: vi.fn(),
      update: vi.fn(),
      archive: vi.fn(),
    };

    render(<AgentList projectId={projectId} apiClient={apiClient} />);
    await screen.findByText('Nenhum agent encontrado para este projeto.');

    fireEvent.click(screen.getByRole('button', { name: 'Abrir formulário de criação de agent' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Nome do agent' }), {
      target: { value: 'release-agent' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'Descrição do agent' }), {
      target: { value: 'Prepara releases com revisão humana.' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Criar agent' }));

    await waitFor(() => expect(apiClient.create).toHaveBeenCalledWith({
      project_id: projectId,
      name: 'release-agent',
      description: 'Prepara releases com revisão humana.',
    }));
    expect(await screen.findByText('release-agent')).toBeInTheDocument();
  });
});
