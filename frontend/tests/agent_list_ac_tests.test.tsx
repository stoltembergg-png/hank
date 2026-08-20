import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AgentList } from '@/components/AgentList';
import { defaultAgentApi } from '@/api/agents';
import { AgentSummary, AgentStatus } from '@/types/agent';

// Mock the defaultAgentApi
vi.mock('@/api/agents', () => ({
  defaultAgentApi: {
    list: vi.fn(),
  },
}));

const mockAgents: AgentSummary[] = [
  {
    id: 'agt_1',
    project_id: 'prj_1',
    name: 'worker-1',
    description: 'First worker',
    status: 'active',
    personality: {
      name: 'Default',
      description: null,
      traits: ['helpful', 'accurate'],
      communication_style: 'technical',
    },
    created_at: '2026-01-01T00:00:00.000Z',
    updated_at: '2026-01-01T00:00:00.000Z',
  },
  {
    id: 'agt_2',
    project_id: 'prj_1',
    name: 'worker-2',
    description: null,
    status: 'inactive',
    personality: {
      name: 'Default',
      description: null,
      traits: ['helpful', 'accurate'],
      communication_style: 'technical',
    },
    created_at: '2026-01-02T00:00:00.000Z',
    updated_at: '2026-01-02T00:00:00.000Z',
  },
];

describe('AgentList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state initially', () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: [],
      total: 0,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" />);

    expect(screen.getByText('Carregando agents...')).toBeInTheDocument();
  });

  it('renders empty state when no agents found', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: [],
      total: 0,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" />);

    await waitFor(() => {
      expect(screen.getByText('Nenhum agent encontrado para este projeto.')).toBeInTheDocument();
    });
  });

  it('renders error state and retry button', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Network error'));

    render(<AgentList projectId="prj_1" />);

    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeInTheDocument();
    });

    expect(screen.getByText('Tentar novamente')).toBeInTheDocument();
  });

  it('renders agents table with data', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: mockAgents,
      total: 2,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" />);

    await waitFor(() => {
      expect(screen.getByText('worker-1')).toBeInTheDocument();
      expect(screen.getByText('worker-2')).toBeInTheDocument();
    });

    expect(screen.getByText('active')).toBeInTheDocument();
    expect(screen.getByText('inactive')).toBeInTheDocument();
  });

  it('shows pagination when multiple pages', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: mockAgents,
      total: 25,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" pageSize={10} />);

    await waitFor(() => {
      expect(screen.getByText('Página 1 de 3')).toBeInTheDocument();
    });

    expect(screen.getByText('Próxima')).not.toBeDisabled();
    expect(screen.getByText('Anterior')).toBeDisabled();
  });

  it('navigates to next page on click', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>)
      .mockResolvedValueOnce({
        agents: mockAgents,
        total: 25,
        limit: 10,
        offset: 0,
      })
      .mockResolvedValueOnce({
        agents: mockAgents,
        total: 25,
        limit: 10,
        offset: 10,
      });

    render(<AgentList projectId="prj_1" pageSize={10} />);

    await waitFor(() => {
      expect(screen.getByText('Página 1 de 3')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Próxima'));

    await waitFor(() => {
      expect(screen.getByText('Página 2 de 3')).toBeInTheDocument();
    });
  });

  it('disables previous button on first page', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: mockAgents,
      total: 25,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" pageSize={10} />);

    await waitFor(() => {
      const prevButton = screen.getByRole('button', { name: /anterior/i });
      expect(prevButton).toBeDisabled();
    });
  });

  it('enables next button on first page when multiple pages exist', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: mockAgents,
      total: 25,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" pageSize={10} />);

    await waitFor(() => {
      const nextButton = screen.getByRole('button', { name: /próxima/i });
      expect(nextButton).not.toBeDisabled();
    });
  });

  it('navigates to last page and disables next button', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>)
      .mockResolvedValueOnce({
        agents: mockAgents,
        total: 25,
        limit: 10,
        offset: 0,
      })
      .mockResolvedValueOnce({
        agents: mockAgents,
        total: 25,
        limit: 10,
        offset: 10,
      })
      .mockResolvedValueOnce({
        agents: mockAgents,
        total: 25,
        limit: 10,
        offset: 20,
      });

    render(<AgentList projectId="prj_1" pageSize={10} />);

    await waitFor(() => {
      expect(screen.getByText('Página 1 de 3')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /próxima/i }));

    await waitFor(() => {
      expect(screen.getByText('Página 2 de 3')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /próxima/i }));

    await waitFor(() => {
      expect(screen.getByText('Página 3 de 3')).toBeInTheDocument();
    });

    const nextButton = screen.getByRole('button', { name: /próxima/i });
    expect(nextButton).toBeDisabled();
  });

  it('renders agent status with correct styling classes', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: mockAgents,
      total: 2,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" />);

    await waitFor(() => {
      const activeStatus = screen.getByText('active');
      expect(activeStatus).toHaveClass('agent-status--active');

      const inactiveStatus = screen.getByText('inactive');
      expect(inactiveStatus).toHaveClass('agent-status--inactive');
    });
  });

  it('shows total count in header', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: mockAgents,
      total: 42,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" />);

    await waitFor(() => {
      expect(screen.getByText('Agents (42)')).toBeInTheDocument();
    });
  });

  it('renders personality name column', async () => {
    (defaultAgentApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      agents: mockAgents,
      total: 2,
      limit: 10,
      offset: 0,
    });

    render(<AgentList projectId="prj_1" />);

    await waitFor(() => {
      const personalityCells = screen.getAllByText('Default');
      expect(personalityCells.length).toBeGreaterThanOrEqual(2);
    });
  });
});