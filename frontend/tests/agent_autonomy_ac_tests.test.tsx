import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AutonomyPage } from '@/agents/builder/autonomy/AutonomyPage';
import {
  AutonomyDecision,
  AutonomyLevel,
  AutonomyOperation,
  AutonomyPolicySnapshot,
  AutonomyApiClient,
} from '@/api/agent-autonomy';

const decisions: Record<AutonomyOperation, AutonomyDecision> = {
  read_data: 'allow',
  execute_safe_tool: 'allow',
  execute_stateful_tool: 'require_human_approval',
  spawn_sub_agent: 'require_human_approval',
  create_workflow: 'require_human_approval',
  modify_skill: 'deny',
  access_external_network: 'deny',
  modify_system_config: 'require_human_approval',
};

const snapshot: AutonomyPolicySnapshot = {
  policy: {
    schema_version: 1,
    level: 'l1_assisted',
    allow_subagents: false,
    allow_workflow_creation: false,
    allow_skill_modification: false,
    allow_network_access: false,
    max_consecutive_autonomous_steps: 10,
  },
  decisions,
  updated_at: '2026-01-01T00:00:00.000Z',
};

function createApiClient(): AutonomyApiClient {
  return {
    get: vi.fn().mockResolvedValue(snapshot),
    update: vi.fn().mockResolvedValue(snapshot),
  };
}

describe('AutonomyPage', () => {
  let apiClient: AutonomyApiClient;
  const onBack = vi.fn();
  const onSaved = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    apiClient = createApiClient();
  });

  it('renders loading state while autonomy policy is fetched', () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {}));

    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    expect(screen.getByText('Carregando política de autonomia...')).toBeInTheDocument();
  });

  it('renders current level, consequences, decisions, and approval indicator', async () => {
    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Nível de autonomia' })).toHaveValue('l1_assisted');
    });

    expect(screen.getByRole('heading', { name: 'L1 — Assistido' })).toBeInTheDocument();
    expect(screen.getByText('read_data: allow')).toBeInTheDocument();
    expect(screen.getByText('execute_stateful_tool: require_human_approval')).toBeInTheDocument();
    expect(screen.getByText(/escalações exigem aprovação humana/i)).toBeInTheDocument();
    expect(screen.getByText(/LLM não pode alterar esta política/i)).toBeInTheDocument();
  });

  it('rejects unauthorized escalation before calling the service', async () => {
    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Nível de autonomia' })).toHaveValue('l1_assisted');
    });

    fireEvent.change(screen.getByRole('combobox', { name: 'Nível de autonomia' }), {
      target: { value: 'l2_semi_autonomous' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de autonomia' }));

    expect(screen.getByRole('alert')).toHaveTextContent(/aprovação humana explícita/i);
    expect(apiClient.update).not.toHaveBeenCalled();
  });

  it('submits an escalation only with bounded approval metadata', async () => {
    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
        onSaved={onSaved}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Nível de autonomia' })).toHaveValue('l1_assisted');
    });

    fireEvent.change(screen.getByRole('combobox', { name: 'Nível de autonomia' }), {
      target: { value: 'l2_semi_autonomous' },
    });
    fireEvent.change(screen.getByLabelText('Approver ID'), { target: { value: 'human-1' } });
    fireEvent.change(screen.getByLabelText('Motivo da aprovação'), {
      target: { value: 'Approved bounded project workflow' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de autonomia' }));

    await waitFor(() => {
      expect(apiClient.update).toHaveBeenCalledWith({
        project_id: 'prj_1',
        agent_id: 'agt_1',
        policy: {
          ...snapshot.policy,
          level: 'l2_semi_autonomous',
          allow_subagents: true,
          allow_workflow_creation: true,
          allow_skill_modification: false,
          allow_network_access: false,
          max_consecutive_autonomous_steps: 50,
        },
        approval: {
          approver_id: 'human-1',
          reason: 'Approved bounded project workflow',
          expires_at: null,
        },
        expected_version: '2026-01-01T00:00:00.000Z',
      });
    });

    expect(onSaved).toHaveBeenCalledWith(snapshot);
    expect(onBack).toHaveBeenCalled();
  });

  it('allows a downgrade without approval and preserves reversibility', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...snapshot,
      policy: { ...snapshot.policy, level: 'l3_autonomous' },
    });

    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Nível de autonomia' })).toHaveValue('l3_autonomous');
    });

    fireEvent.change(screen.getByRole('combobox', { name: 'Nível de autonomia' }), {
      target: { value: 'l1_assisted' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de autonomia' }));

    await waitFor(() => {
      expect(apiClient.update).toHaveBeenCalled();
    });
    expect((apiClient.update as ReturnType<typeof vi.fn>).mock.calls[0][0]).not.toHaveProperty('approval');
  });

  it('rejects malformed policy flags and does not render active controls', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...snapshot,
      policy: { ...snapshot.policy, level: 'l1_assisted', allow_network_access: true },
    });

    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/inválida|L1/i);
    });
    expect(screen.queryByRole('combobox', { name: 'Nível de autonomia' })).not.toBeInTheDocument();
  });

  it('maps stale update errors to a visible rollback state', async () => {
    (apiClient.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('stale version'));

    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Nível de autonomia' })).toHaveValue('l1_assisted');
    });

    fireEvent.change(screen.getByRole('combobox', { name: 'Nível de autonomia' }), {
      target: { value: 'l0_none' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de autonomia' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/modificada por outro processo/i);
    });
    expect(screen.getByRole('combobox', { name: 'Nível de autonomia' })).toHaveValue('l0_none');
    expect(onBack).not.toHaveBeenCalled();
  });

  it('shows unsupported state without enabling autonomous behavior', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue(null);

    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText(/Nenhum serviço de autonomia disponível/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/nenhuma elevação automática/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Salvar política de autonomia' })).not.toBeInTheDocument();
  });

  it('confirms cancellation and exposes accessible security metadata', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);

    render(
      <AutonomyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Nível de autonomia' })).toHaveValue('l1_assisted');
    });

    expect(screen.getByRole('form', { name: 'Formulário de autonomia' })).toBeInTheDocument();
    expect(screen.getByText(/sem autoelevação silenciosa/i)).toBeInTheDocument();
    fireEvent.change(screen.getByRole('combobox', { name: 'Nível de autonomia' }), {
      target: { value: 'l0_none' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));

    expect(confirmSpy).toHaveBeenCalled();
    expect(onBack).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });
});