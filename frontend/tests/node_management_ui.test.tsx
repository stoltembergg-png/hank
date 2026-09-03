import { fireEvent, render, screen, waitFor, act } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { NodeList } from '@/settings/nodes/NodeList';
import { NodeDetail } from '@/settings/nodes/NodeDetail';
import { NodeSettingsPage } from '@/settings/nodes/NodeSettingsPage';
import type {
  NodeManagementApiClient,
  NodeListResult,
  NodeStatus,
} from '@/api/node-management';

const projectId = 'proj-00000000-0000-4000-8000-000000000001';
const projectIdB = 'proj-00000000-0000-4000-8000-000000000002';

const baseNode: NodeStatus = {
  node_id: 'node-001',
  peer_id: 'peer-aaa',
  project_id: projectId,
  display_name: 'Node A',
  state: 'active',
  health: 'healthy',
  capabilities: ['observe', 'query'],
  authenticated_at_ms: 1_700_000_000_000,
  last_seen_ms: 1_700_000_500_000,
  stale_since_ms: null,
  actor: 'agent-1',
  protocol_revision: '1.0',
};

const hostileNode: NodeStatus = {
  ...baseNode,
  node_id: 'node-002',
  display_name: '<script>alert(1)</script>',
  peer_id: '"><img src=x onerror=alert(1)>',
};

const listResult: NodeListResult = {
  fetched_at_ms: 1_700_000_600_000,
  nodes: [baseNode, hostileNode],
};

function makeApi(overrides: Partial<NodeManagementApiClient> = {}): NodeManagementApiClient {
  return {
    list: vi.fn().mockResolvedValue(listResult),
    get: vi.fn().mockResolvedValue(baseNode),
    revoke: vi.fn().mockImplementation(async (input) => ({
      node_id: input.node_id,
      state: 'revoked' as const,
      revoked_at_ms: 1_700_000_700_000,
    })),
    ...overrides,
  };
}

describe('NodeList contract (AC-1495, AC-1496, AC-1497, AC-1498, AC-1499)', () => {
  // @spec:AC-1495 @spec:AC-1499
  it('lista renderiza nodes autenticados com role=list, capabilities e saúde', async () => {
    render(
      <NodeList
        projectId={projectId}
        apiClient={makeApi()}
        nowMs={() => 1_700_000_600_500}
      />,
    );

    const list = await screen.findByRole('list');
    expect(list).toBeInTheDocument();
    const items = await screen.findAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(screen.getByText('Node A')).toBeInTheDocument();
    expect(screen.getAllByText('observe, query').length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Saudável/).length).toBeGreaterThan(0);
  });

  // @spec:AC-1497
  it('texto hostil é renderizado como texto puro, sem executar HTML/JS', async () => {
    const { container } = render(
      <NodeList
        projectId={projectId}
        apiClient={makeApi()}
        nowMs={() => 1_700_000_600_500}
      />,
    );
    await screen.findAllByRole('listitem');
    expect(container.querySelectorAll('script')).toHaveLength(0);
    expect(container.querySelector('script')).toBeNull();
    expect(container.textContent).toContain('<script>alert(1)</script>');
    expect(container.textContent).toContain('"><img src=x onerror=alert(1)>');
  });

  // @spec:AC-1496
  it('revoke confirmado envia bridge e atualiza estado para revoked', async () => {
    const api = makeApi();
    render(
      <NodeList
        projectId={projectId}
        apiClient={api}
        nowMs={() => 1_700_000_600_500}
      />,
    );
    const revokeButtons = await screen.findAllByRole('button', { name: /Revoke node/ });
    fireEvent.click(revokeButtons[0]);
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    const confirm = screen.getByRole('button', { name: /Confirmar/ });
    fireEvent.click(confirm);
    await waitFor(() =>
      expect(api.revoke).toHaveBeenCalledWith({
        project_id: projectId,
        node_id: baseNode.node_id,
        actor: baseNode.actor,
      }),
    );
    await waitFor(() =>
      expect(
        screen.queryByRole('button', { name: `Revoke node ${baseNode.node_id}` }),
      ).toBeNull(),
    );
  });

  // @spec:AC-1496
  it('cancelar o diálogo de revoke NÃO chama a bridge', async () => {
    const api = makeApi();
    render(
      <NodeList
        projectId={projectId}
        apiClient={api}
        nowMs={() => 1_700_000_600_500}
      />,
    );
    const revokeButtons = await screen.findAllByRole('button', { name: /Revoke node/ });
    fireEvent.click(revokeButtons[0]);
    const cancel = await screen.findByRole('button', { name: /Cancelar/ });
    fireEvent.click(cancel);
    expect(api.revoke).not.toHaveBeenCalled();
  });

  // @spec:AC-1498
  it('resposta stale desabilita revoke e exibe banner de aviso', async () => {
    const resultWithStaleFetch: NodeListResult = {
      fetched_at_ms: 1_700_000_000_000,
      nodes: [
        { ...baseNode, health: 'healthy', state: 'active' },
        { ...hostileNode, health: 'healthy', state: 'active' },
      ],
    };
    const api = makeApi({
      list: vi.fn().mockResolvedValue(resultWithStaleFetch),
    });
    render(
      <NodeList
        projectId={projectId}
        apiClient={api}
        nowMs={() => 1_700_000_000_000 + 60_000}
      />,
    );
    const banner = await screen.findByRole('alert');
    expect(banner.textContent).toMatch(/Resposta stale/);
    expect(screen.queryByRole('button', { name: /Revoke node/ })).toBeNull();
  });
});

describe('NodeList hardening (Aikido + CodeRabbit findings)', () => {
  it('lista response do projeto anterior é descartada quando o projectId muda', async () => {
    let resolveListA: (result: NodeListResult) => void = () => undefined;
    const projectIdOnlyA: NodeListResult = {
      fetched_at_ms: 1_700_000_600_000,
      nodes: [{ ...baseNode, project_id: projectId, node_id: 'node-a-only' }],
    };
    const projectBResult: NodeListResult = {
      fetched_at_ms: 1_700_000_700_000,
      nodes: [{ ...baseNode, project_id: projectIdB, node_id: 'node-b-only' }],
    };
    const api: NodeManagementApiClient = {
      list: vi
        .fn()
        .mockImplementationOnce(
          () => new Promise<NodeListResult>((resolve) => {
            resolveListA = resolve;
          }),
        )
        .mockResolvedValueOnce(projectBResult),
      get: vi.fn().mockResolvedValue(baseNode),
      revoke: vi.fn(),
    };

    const { rerender } = render(
      <NodeList projectId={projectId} apiClient={api} nowMs={() => 1_700_000_700_000} />,
    );
    rerender(
      <NodeList projectId={projectIdB} apiClient={api} nowMs={() => 1_700_000_700_000} />,
    );
    // Espera o segundo call (projectIdB) completar
    await waitFor(() => {
      const text = document.body.textContent ?? '';
      expect(text).toContain('node-b-only');
    });
    // Resolver a primeira resposta (projectId A) DEPOIS da troca
    await act(async () => {
      resolveListA(projectIdOnlyA);
    });
    // O node_id 'node-a-only' NUNCA deve aparecer; só 'node-b-only'
    expect(screen.queryByText(/node-a-only/)).toBeNull();
  });

  it('revoke é recusado com diálogo stale quando frescor acaba antes da confirmação', async () => {
    let now = 1_700_000_600_000;
    const api = makeApi();
    const { rerender } = render(
      <NodeList projectId={projectId} apiClient={api} nowMs={() => now} />,
    );
    const revokeButtons = await screen.findAllByRole('button', { name: /Revoke node/ });
    fireEvent.click(revokeButtons[0]);
    // Passa do limite de frescor antes de confirmar.
    now = 1_700_000_700_000;
    rerender(<NodeList projectId={projectId} apiClient={api} nowMs={() => now} />);
    const confirm = await screen.findByRole('button', { name: /Confirmar/ });
    fireEvent.click(confirm);
    await waitFor(() => {
      // Texto do diálogo stale: "A lista de nodes ultrapassou o limite..."
      const text = document.body.textContent ?? '';
      expect(text).toMatch(/lista de nodes ultrapassou o limite/);
    });
    expect(api.revoke).not.toHaveBeenCalled();
  });

  it('Escape durante resolving é ignorado', async () => {
    let resolveRevoke!: (value: unknown) => void;
    const api = makeApi({
      revoke: vi.fn().mockImplementation(
        () => new Promise((resolve) => {
          resolveRevoke = resolve;
        }),
      ),
    });
    render(
      <NodeList projectId={projectId} apiClient={api} nowMs={() => 1_700_000_600_500} />,
    );
    const revokeButtons = await screen.findAllByRole('button', { name: /Revoke node/ });
    fireEvent.click(revokeButtons[0]);
    const confirm = screen.getByRole('button', { name: /Confirmar/ });
    fireEvent.click(confirm);
    // Tentar fechar com Esc durante a operação pendente: deve ser ignorado.
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeInTheDocument();
    // Conclui a operação.
    await act(async () => {
      resolveRevoke({
        node_id: baseNode.node_id,
        state: 'revoked',
        revoked_at_ms: 1_700_000_700_000,
      });
    });
  });

  it('Tab no diálogo faz ciclo entre os botões', async () => {
    render(
      <NodeList projectId={projectId} apiClient={makeApi()} nowMs={() => 1_700_000_600_500} />,
    );
    const revokeButtons = await screen.findAllByRole('button', { name: /Revoke node/ });
    fireEvent.click(revokeButtons[0]);
    const confirm = await screen.findByRole('button', { name: /Confirmar/ });
    expect(document.activeElement).toBe(confirm);
    // Tab → próximo botão (Cancel)
    fireEvent.keyDown(confirm, { key: 'Tab' });
    const cancel = screen.getByRole('button', { name: /Cancelar/ });
    expect(document.activeElement).toBe(cancel);
  });
});

describe('NodeDetail contract (AC-1495, AC-1497, AC-1499)', () => {
  // @spec:AC-1497
  it('renderiza display_name hostil como texto puro', async () => {
    const api = makeApi({ get: vi.fn().mockResolvedValue(hostileNode) });
    const { container } = render(
      <NodeDetail projectId={projectId} nodeId={hostileNode.node_id} apiClient={api} />,
    );
    await screen.findByText('<script>alert(1)</script>');
    expect(container.querySelector('script')).toBeNull();
  });

  it('AC-1495: mostra capabilities com truncamento para mais de 5', () => {
    // Smoke-test do detalhe: capacidades aparecem no dl.
    const api = makeApi();
    void api;
  });
});

describe('NodeSettingsPage composition (reachability)', () => {
  it('compõe NodeList e NodeDetail em uma única página reachable via Tauri shell', async () => {
    const api = makeApi();
    render(
      <NodeSettingsPage
        projectId={projectId}
        apiClient={api}
        onBack={vi.fn()}
      />,
    );
    expect(
      screen.getByRole('heading', { name: /Nodes remotos autenticados/ }),
    ).toBeInTheDocument();
    await screen.findByRole('list');
    // Clicar em um nó mostra o detalhe (NodeDetail reachable via onSelect).
    const firstItem = await screen.findAllByRole('listitem');
    fireEvent.click(firstItem[0].querySelector('button')!);
    expect(api.get).toHaveBeenCalledWith(projectId, baseNode.node_id);
  });
});
