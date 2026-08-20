import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AgentIdentityPage } from '@/agents/builder/identity/AgentIdentityPage';
import { defaultAgentApi } from '@/api/agents';
import { AgentSummary, AgentStatus } from '@/types/agent';

// Mock the defaultAgentApi
vi.mock('@/api/agents', () => ({
  defaultAgentApi: {
    get: vi.fn(),
    update: vi.fn(),
  },
}));

const mockAgent: AgentSummary = {
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
};

describe('AgentIdentityPage', () => {
  const mockOnBack = vi.fn();
  const mockOnSaved = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (defaultAgentApi.get as ReturnType<typeof vi.fn>).mockResolvedValue(mockAgent);
    (defaultAgentApi.update as ReturnType<typeof vi.fn>).mockResolvedValue({
      agent: mockAgent,
      event_id: 'evt_1',
      correlation_id: null,
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it('renders loading state initially', () => {
    (defaultAgentApi.get as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {}));

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    expect(screen.getByText('Carregando agent...')).toBeInTheDocument();
  });

  it('renders error state when agent not found', async () => {
    (defaultAgentApi.get as ReturnType<typeof vi.fn>).mockResolvedValue(null);

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByText('Agent não encontrado')).toBeInTheDocument();
    });

    expect(screen.getByText('Tentar novamente')).toBeInTheDocument();
  });

  it('renders error state on fetch failure', async () => {
    (defaultAgentApi.get as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Network error'));

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByText(/carregando agent/i)).toBeInTheDocument();
    });
  });

  it('renders form with agent data when loaded', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    expect(screen.getByDisplayValue('First worker')).toBeInTheDocument();
  });

  it('shows agent meta information', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    expect(screen.getByText('active')).toBeInTheDocument();
    expect(screen.getByText('agt_1')).toBeInTheDocument();
    expect(screen.getByText('prj_1')).toBeInTheDocument();
  });

  it('validates required name field', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    // Clear the name
    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: '' } });

    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      expect(screen.getByText('Nome é obrigatório')).toBeInTheDocument();
    });
  });

  it('validates max name length', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    const longName = 'a'.repeat(121);
    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: longName } });

    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      expect(screen.getByText('Nome deve ter no máximo 120 caracteres')).toBeInTheDocument();
    });
  });

  it('validates max description length', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    const longDesc = 'a'.repeat(501);
    fireEvent.change(screen.getByLabelText('Descrição'), { target: { value: longDesc } });

    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      expect(screen.getByText('Descrição deve ter no máximo 500 caracteres')).toBeInTheDocument();
    });
  });

  it('calls update API with correct payload on valid submit', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} onSaved={mockOnSaved} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: 'updated-name' } });
    fireEvent.change(screen.getByLabelText('Descrição'), { target: { value: 'updated description' } });

    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      expect(defaultAgentApi.update).toHaveBeenCalledWith(
        expect.objectContaining({
          project_id: 'prj_1',
          agent_id: 'agt_1',
          name: 'updated-name',
          description: 'updated description',
          expected_version: '2026-01-01T00:00:00.000Z',
        })
      );
    });

    expect(mockOnSaved).toHaveBeenCalled();
    expect(mockOnBack).toHaveBeenCalled();
  });

  it('does not call update when no changes made', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Cancelar'));

    expect(defaultAgentApi.update).not.toHaveBeenCalled();
    expect(mockOnBack).toHaveBeenCalled();
  });

  it('handles stale version error', async () => {
    (defaultAgentApi.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('stale version'));

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: 'updated-name' } });
    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      expect(screen.getByText('O agent foi modificado por outro processo. Recarregue e tente novamente.')).toBeInTheDocument();
    });
  });

  it('handles archived agent error', async () => {
    (defaultAgentApi.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('archived agent'));

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: 'updated-name' } });
    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      expect(screen.getByText('Não é possível editar um agent arquivado ou inativo.')).toBeInTheDocument();
    });
  });

  it('handles permission error', async () => {
    (defaultAgentApi.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('forbidden'));

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: 'updated-name' } });
    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      expect(screen.getByText('Você não tem permissão para editar este agent.')).toBeInTheDocument();
    });
  });

  it('disables form fields when agent is inactive', async () => {
    (defaultAgentApi.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...mockAgent,
      status: 'inactive',
    });

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    expect(screen.getByLabelText('Nome *')).toBeDisabled();
    expect(screen.getByLabelText('Descrição')).toBeDisabled();
    expect(screen.getByText('Salvar alterações')).toBeDisabled();
    expect(screen.getByText('Este agent está arquivado e não pode ser editado.')).toBeInTheDocument();
  });

  it('disables form fields when agent is suspended', async () => {
    (defaultAgentApi.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...mockAgent,
      status: 'suspended',
    });

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    expect(screen.getByLabelText('Nome *')).toBeDisabled();
    expect(screen.getByLabelText('Descrição')).toBeDisabled();
    expect(screen.getByText('Salvar alterações')).toBeDisabled();
  });

  it('shows character count for name field', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    await waitFor(() => {
      const charCount = screen.getByTestId('name-char-count');
      expect(charCount).toHaveTextContent(/8\/120/);
    });
  });

  it('shows character count for description field', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('First worker')).toBeInTheDocument();
    });

    await waitFor(() => {
      const charCount = screen.getByTestId('desc-char-count');
      expect(charCount).toHaveTextContent(/12\/500/);
    });
  });

  it('calls onBack when cancel clicked without changes', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Cancelar'));

    expect(mockOnBack).toHaveBeenCalled();
  });

  it('has accessible form labels and hints', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    expect(screen.getByLabelText('Nome *')).toBeInTheDocument();
    expect(screen.getByLabelText('Descrição')).toBeInTheDocument();
    expect(screen.getByText('Nome único do agent dentro do projeto')).toBeInTheDocument();
    expect(screen.getByText('Descrição opcional do propósito do agent')).toBeInTheDocument();
  });

  it('has proper ARIA attributes on error state', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: '' } });
    fireEvent.click(screen.getByText('Salvar alterações'));

    await waitFor(() => {
      const errorDiv = screen.getByRole('alert');
      expect(errorDiv).toBeInTheDocument();
    });
  });

  it('has loading state on submit button', async () => {
    let resolveUpdate: (value: unknown) => void;
    const updatePromise = new Promise((resolve) => {
      resolveUpdate = resolve;
    });
    (defaultAgentApi.update as ReturnType<typeof vi.fn>).mockReturnValue(updatePromise);

    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Nome *'), { target: { value: 'updated-name' } });
    fireEvent.click(screen.getByText('Salvar alterações'));

    expect(screen.getByText('Salvando...')).toBeInTheDocument();
    expect(screen.queryByText('Salvar alterações')).not.toBeInTheDocument();

    resolveUpdate!({ agent: mockAgent, event_id: 'evt_1', correlation_id: null });
  });

  it('displays status badge with correct styling classes', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    const activeStatus = screen.getByText('active');
    expect(activeStatus).toHaveClass('agent-status--active');
  });

  it('has proper form structure', async () => {
    render(<AgentIdentityPage projectId="prj_1" agentId="agt_1" onBack={mockOnBack} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue('worker-1')).toBeInTheDocument();
    });

    const form = screen.getByTestId('agent-identity-form');
    expect(form).toBeInTheDocument();
    expect(form).toHaveAttribute('noValidate', '');
  });
});