import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { NodeList } from '@/settings/nodes/NodeList';
import { NodeDetail } from '@/settings/nodes/NodeDetail';
import type {
  NodeManagementApiClient,
  NodeListResult,
  NodeStatus,
} from '@/api/node-management';

const projectId = 'proj-00000000-0000-4000-8000-000000000001';

const activeNode: NodeStatus = {
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
  ...activeNode,
  node_id: 'node-002',
  display_name: '<script>alert(1)</script>',
  peer_id: '"><img src=x onerror=alert(1)>',
};

const listResult: NodeListResult = {
  fetched_at_ms: 1_700_000_600_000,
  nodes: [activeNode, hostileNode],
};

function makeApi(overrides: Partial<NodeManagementApiClient> = {}): NodeManagementApiClient {
  return {
    list: vi.fn().mockResolvedValue(listResult),
    get: vi.fn().mockResolvedValue(activeNode),
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
  it('lista renderiza nodes autenticados com role=list', async () => {
    render(<NodeList projectId={projectId} apiClient={makeApi()} nowMs={() => 1_700_000_600_500} />);

    const list = await screen.findByRole('list');
    expect(list).toBeInTheDocument();
    const items = await screen.findAllByRole('listitem');
    expect(items).toHaveLength(2);
    expect(screen.getByText('Node A')).toBeInTheDocument();
  });

  it('AC-1497: texto hostil é renderizado como texto puro, sem executar HTML/JS', async () => {
    const { container } = render(
      <NodeList projectId={projectId} apiClient={makeApi()} nowMs={() => 1_700_000_600_500} />,
    );
    await screen.findAllByRole('listitem');
    // Nenhum <script> criado a partir do display_name
    expect(container.querySelectorAll('script')).toHaveLength(0);
    expect(container.querySelector('script')).toBeNull();
    // O conteúdo aparece como texto, não como nó HTML
    expect(container.textContent).toContain('<script>alert(1)</script>');
    expect(container.textContent).toContain('"><img src=x onerror=alert(1)>');
  });

  // @spec:AC-1496
  it('revoke confirmado envia bridge e atualiza estado para revoked', async () => {
    const api = makeApi();
    render(<NodeList projectId={projectId} apiClient={api} nowMs={() => 1_700_000_600_500} />);
    const revokeButtons = await screen.findAllByRole('button', { name: /Revoke node/ });
    fireEvent.click(revokeButtons[0]);
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    const confirm = screen.getByRole('button', { name: /Confirmar/ });
    fireEvent.click(confirm);
    await waitFor(() => expect(api.revoke).toHaveBeenCalledWith({
      project_id: projectId,
      node_id: activeNode.node_id,
      actor: activeNode.actor,
    }));
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: `Revoke node ${activeNode.node_id}` })).toBeNull(),
    );
  });

  // @spec:AC-1496
  it('cancelar o diálogo de revoke NÃO chama a bridge', async () => {
    const api = makeApi();
    render(<NodeList projectId={projectId} apiClient={api} nowMs={() => 1_700_000_600_500} />);
    const revokeButtons = await screen.findAllByRole('button', { name: /Revoke node/ });
    fireEvent.click(revokeButtons[0]);
    const cancel = await screen.findByRole('button', { name: /Cancelar/ });
    fireEvent.click(cancel);
    expect(api.revoke).not.toHaveBeenCalled();
  });

  // @spec:AC-1498
  it('resposta stale desabilita revoke e exibe banner de aviso', async () => {
    const resultWithStaleFetch: NodeListResult = {
      fetched_at_ms: 1_700_000_000_000, // fetched 60s ago
      nodes: [
        { ...activeNode, health: 'healthy', state: 'active' },
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
    // Resposta stale desabilita revoke completamente.
    expect(screen.queryByRole('button', { name: /Revoke node/ })).toBeNull();
  });
});

describe('NodeDetail contract (AC-1495, AC-1497, AC-1499)', () => {
  // @spec:AC-1497
  it('renderiza display_name hostil como texto puro', async () => {
    const api = makeApi({ get: vi.fn().mockResolvedValue(hostileNode) });
    const { container } = render(<NodeDetail projectId={projectId} nodeId={hostileNode.node_id} apiClient={api} />);
    await screen.findByText('<script>alert(1)</script>');
    expect(container.querySelector('script')).toBeNull();
  });
});
