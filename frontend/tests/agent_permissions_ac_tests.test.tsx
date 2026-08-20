import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { PermissionsPage } from '@/agents/builder/permissions/PermissionsPage';
import {
  PermissionEffect,
  PermissionRule,
  PermissionScope,
  ToolPermissionApiClient,
  ToolPermissionPolicySnapshot,
} from '@/api/agent-tool-permissions';

const existingRule: PermissionRule = {
  capability: { resource: 'tool', action: 'invoke', scope: 'tool:calendar' },
  effect: 'ask',
  scope: 'project',
  scope_id: 'prj_1',
  expires_at: null,
};

const snapshot: ToolPermissionPolicySnapshot = {
  policy: {
    schema_version: 1,
    default_effect: 'deny',
    rules: [existingRule],
  },
  updated_at: '2026-01-01T00:00:00.000Z',
};

function createApiClient(): ToolPermissionApiClient {
  return {
    get: vi.fn().mockResolvedValue(snapshot),
    update: vi.fn().mockResolvedValue(snapshot),
  };
}

describe('PermissionsPage', () => {
  let apiClient: ToolPermissionApiClient;
  const onBack = vi.fn();
  const onSaved = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    apiClient = createApiClient();
  });

  it('renders loading state while permissions are fetched', () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {}));

    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    expect(screen.getByText('Carregando permissões...')).toBeInTheDocument();
  });

  it('renders default deny and the effective existing rule', async () => {
    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Políticas de permissões do Agent')).toBeInTheDocument();
    });

    expect(screen.getByText('Default: deny')).toBeInTheDocument();
    expect(screen.getByText('tool:invoke')).toBeInTheDocument();
    expect(screen.getByText(/Regra efetiva: ask/i)).toBeInTheDocument();
    expect(screen.getByText(/Aprovação humana necessária/i)).toBeInTheDocument();
  });

  it('adds a safe rule and submits the typed policy boundary', async () => {
    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
        onSaved={onSaved}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Default: deny')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Recurso'), { target: { value: 'tool' } });
    fireEvent.change(screen.getByLabelText('Ação'), { target: { value: 'read' } });
    fireEvent.change(screen.getByLabelText('Efeito'), { target: { value: 'allow' } });
    fireEvent.change(screen.getByLabelText('Escopo'), { target: { value: 'agent' } });
    fireEvent.change(screen.getByLabelText('ID do escopo'), { target: { value: 'agt_1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar regra' }));

    expect(screen.getByText('tool:read')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Salvar permissões' }));

    await waitFor(() => {
      expect(apiClient.update).toHaveBeenCalledWith({
        project_id: 'prj_1',
        agent_id: 'agt_1',
        policy: {
          schema_version: 1,
          default_effect: 'deny',
          rules: [
            existingRule,
            {
              capability: { resource: 'tool', action: 'read', scope: 'agt_1' },
              effect: 'allow',
              scope: 'agent',
              scope_id: 'agt_1',
              expires_at: null,
            },
          ],
        },
        expected_version: '2026-01-01T00:00:00.000Z',
      });
    });

    expect(onSaved).toHaveBeenCalledWith(snapshot);
    expect(onBack).toHaveBeenCalled();
  });

  it('rejects wildcard and malformed scope identifiers', async () => {
    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Default: deny')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('ID do escopo'), { target: { value: '*' } });
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar regra' }));

    expect(screen.getByRole('alert')).toHaveTextContent(/wildcard|escopo inválido/i);
    expect(screen.queryByText('tool:read')).not.toBeInTheDocument();
  });

  it('requires ask or deny for destructive process execution', async () => {
    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Default: deny')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Recurso'), { target: { value: 'process' } });
    fireEvent.change(screen.getByLabelText('Ação'), { target: { value: 'execute' } });
    fireEvent.change(screen.getByLabelText('Efeito'), { target: { value: 'allow' } });
    fireEvent.change(screen.getByLabelText('ID do escopo'), { target: { value: 'process-1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar regra' }));

    expect(screen.getByRole('alert')).toHaveTextContent(/aprovação explícita/i);
    expect(screen.queryByText('process:execute')).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Efeito'), { target: { value: 'ask' } });
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar regra' }));
    expect(screen.getByText('process:execute')).toBeInTheDocument();
    const approvalMessages = screen.getAllByText(/Aprovação humana necessária/i);
    expect(approvalMessages.length).toBeGreaterThanOrEqual(2);
  });

  it('rejects conflicting duplicate rules before update', async () => {
    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Default: deny')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Recurso'), { target: { value: 'tool' } });
    fireEvent.change(screen.getByLabelText('Ação'), { target: { value: 'invoke' } });
    fireEvent.change(screen.getByLabelText('Efeito'), { target: { value: 'deny' } });
    fireEvent.change(screen.getByLabelText('Escopo'), { target: { value: 'project' } });
    fireEvent.change(screen.getByLabelText('ID do escopo'), { target: { value: 'prj_1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar regra' }));

    expect(screen.getByRole('alert')).toHaveTextContent(/conflitante|duplicada/i);
    expect(apiClient.update).not.toHaveBeenCalled();
  });

  it('rejects malformed policy snapshots and never renders unsafe rules', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...snapshot,
      policy: { ...snapshot.policy, default_effect: 'allow' },
    });

    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/default deny|inválida/i);
    });
    expect(screen.queryByText('tool:invoke')).not.toBeInTheDocument();
  });

  it('maps stale updates to a conflict without navigating', async () => {
    (apiClient.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('stale version'));

    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Default: deny')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Efeito'), { target: { value: 'deny' } });
    fireEvent.change(screen.getByLabelText('ID do escopo'), { target: { value: 'prj_2' } });
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar regra' }));
    fireEvent.click(screen.getByRole('button', { name: 'Salvar permissões' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/modificada por outro processo/i);
    });
    expect(onBack).not.toHaveBeenCalled();
  });

  it('shows unsupported/no-engine state without granting permissions', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue(null);

    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText(/Nenhum Permission Engine disponível/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/default deny permanece implícito/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Salvar permissões' })).not.toBeInTheDocument();
  });

  it('confirms unsaved changes and exposes accessible security metadata', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);

    render(
      <PermissionsPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('Default: deny')).toBeInTheDocument();
    });

    expect(screen.getByRole('heading', { name: 'Políticas de permissões do Agent' })).toBeInTheDocument();
    expect(screen.getByRole('form', { name: 'Formulário de permissões' })).toBeInTheDocument();
    expect(screen.getByText(/sem wildcard padrão/i)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('ID do escopo'), { target: { value: 'prj_2' } });
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar regra' }));
    fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));

    expect(confirmSpy).toHaveBeenCalled();
    expect(onBack).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });
});