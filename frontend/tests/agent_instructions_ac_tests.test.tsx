import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { InstructionsPage } from '@/agents/builder/instructions/InstructionsPage';
import {
  AgentInstructionApiClient,
  AgentInstructionSnapshot,
} from '@/api/agent-instructions';

const snapshot: AgentInstructionSnapshot = {
  layer: 'agent',
  content: 'Use concise answers and state uncertainty explicitly.',
  max_total_bytes: 4_096,
  provenance: 'agent',
  updated_at: '2026-01-01T00:00:00.000Z',
};

function createApiClient(): AgentInstructionApiClient {
  return {
    get: vi.fn().mockResolvedValue(snapshot),
    update: vi.fn().mockResolvedValue(snapshot),
  };
}

describe('InstructionsPage', () => {
  let apiClient: AgentInstructionApiClient;
  const onBack = vi.fn();
  const onSaved = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    apiClient = createApiClient();
  });

  it('renders loading state while instructions are fetched', () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {}));

    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    expect(screen.getByText('Carregando instruções...')).toBeInTheDocument();
  });

  it('renders only the Agent layer with budget and provenance metadata', async () => {
    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('Use concise answers and state uncertainty explicitly.')).toBeInTheDocument();
    });

    expect(screen.getByRole('heading', { name: 'Instruções do Agent' })).toBeInTheDocument();
    expect(screen.getByText('Camada: Agent')).toBeInTheDocument();
    expect(screen.getByText('Proveniência: agent')).toBeInTheDocument();
    expect(screen.getByText(/Budget: 4096 bytes/i)).toBeInTheDocument();
    expect(screen.getByText(/Texto não confiável/i)).toBeInTheDocument();
  });

  it('updates only the Agent instruction layer through the typed service', async () => {
    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
        onSaved={onSaved}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('Use concise answers and state uncertainty explicitly.')).toBeInTheDocument();
    });

    const updated = 'Answer concisely, cite uncertainty, and do not claim unsupported execution.';
    fireEvent.change(screen.getByLabelText('Conteúdo da camada Agent'), { target: { value: updated } });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar instruções' }));

    await waitFor(() => {
      expect(apiClient.update).toHaveBeenCalledWith({
        project_id: 'prj_1',
        agent_id: 'agt_1',
        layer: 'agent',
        content: updated,
        max_total_bytes: 4_096,
        expected_version: '2026-01-01T00:00:00.000Z',
      });
    });

    const payload = (apiClient.update as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(payload.layer).toBe('agent');
    expect(payload).not.toHaveProperty('system');
    expect(payload).not.toHaveProperty('security');
    expect(onSaved).toHaveBeenCalledWith(snapshot);
    expect(onBack).toHaveBeenCalled();
  });

  it('rejects content over the declared budget without truncating', async () => {
    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('Use concise answers and state uncertainty explicitly.')).toBeInTheDocument();
    });

    const oversized = 'x'.repeat(4_097);
    fireEvent.change(screen.getByLabelText('Conteúdo da camada Agent'), { target: { value: oversized } });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar instruções' }));

    expect(screen.getByRole('alert')).toHaveTextContent(/excede o budget de 4096 bytes/i);
    expect(apiClient.update).not.toHaveBeenCalled();
    expect(screen.getByLabelText('Conteúdo da camada Agent')).toHaveValue(oversized);
  });

  it('does not expose a selectable system/security layer or prompt send action', async () => {
    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('Use concise answers and state uncertainty explicitly.')).toBeInTheDocument();
    });

    expect(screen.queryByRole('combobox', { name: /camada/i })).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/system|security/i)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /enviar prompt|executar/i })).not.toBeInTheDocument();
  });

  it('rejects malformed snapshots that attempt to select another layer', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...snapshot,
      layer: 'security',
    });

    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/camada Agent|inválida/i);
    });
    expect(screen.queryByDisplayValue('Use concise answers and state uncertainty explicitly.')).not.toBeInTheDocument();
  });

  it('maps stale updates to a conflict without navigating', async () => {
    (apiClient.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('stale version'));

    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('Use concise answers and state uncertainty explicitly.')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Conteúdo da camada Agent'), { target: { value: 'Changed text' } });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar instruções' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/modificadas por outro processo/i);
    });
    expect(onBack).not.toHaveBeenCalled();
  });

  it('confirms unsaved changes and keeps a plain-text preview', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);

    render(
      <InstructionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('Use concise answers and state uncertainty explicitly.')).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: 'Salvar instruções' })).toBeDisabled();
    fireEvent.change(screen.getByLabelText('Conteúdo da camada Agent'), { target: { value: 'Untrusted <b>text</b>' } });
    expect(screen.getByTestId('instruction-preview')).toHaveTextContent('Untrusted <b>text</b>');
    fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));

    expect(confirmSpy).toHaveBeenCalled();
    expect(onBack).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });
});