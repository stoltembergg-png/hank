import { useEffect, useState, useCallback, useRef } from 'react';
import {
  NodeStatus,
  NodeListResult,
  NodeManagementApiClient,
  NodeRevokeResult,
  defaultNodeManagementApi,
  isStaleResponse,
  STALE_RESPONSE_THRESHOLD_MS,
} from '../../api/node-management';

interface NodeListProps {
  projectId: string;
  apiClient?: NodeManagementApiClient;
  onSelect?: (nodeId: string) => void;
  onError?: (error: string) => void;
  nowMs?: () => number;
}

interface RevokeDialogState {
  node: NodeStatus;
  resolving: boolean;
  error?: string;
}

const HEALTH_LABEL: Record<NodeStatus['health'], string> = {
  healthy: 'Saudável',
  stale: 'Desatualizado',
  unreachable: 'Inacessível',
  unknown: 'Desconhecido',
};

const STATE_LABEL: Record<NodeStatus['state'], string> = {
  active: 'Ativo',
  expired: 'Expirado',
  revoked: 'Revogado',
  unknown: 'Desconhecido',
};

function isAllowedToRevoke(node: NodeStatus): boolean {
  return node.state === 'active' || node.state === 'expired';
}

export function NodeList({
  projectId,
  apiClient = defaultNodeManagementApi,
  onSelect,
  onError,
  nowMs = () => Date.now(),
}: NodeListProps) {
  const [result, setResult] = useState<NodeListResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [dialog, setDialog] = useState<RevokeDialogState | null>(null);
  const confirmButtonRef = useRef<HTMLButtonElement | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const next = await apiClient.list(projectId);
      setResult(next);
    } catch (error) {
      onError?.(error instanceof Error ? error.message : 'Falha ao listar nodes');
    } finally {
      setLoading(false);
    }
  }, [apiClient, projectId, onError]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (dialog && confirmButtonRef.current) {
      confirmButtonRef.current.focus();
    }
  }, [dialog]);

  useEffect(() => {
    if (!dialog) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setDialog(null);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dialog]);

  const handleRevoke = useCallback(
    async (node: NodeStatus) => {
      if (!dialog) return;
      setDialog((current) => (current ? { ...current, resolving: true, error: undefined } : current));
      try {
        const result: NodeRevokeResult = await apiClient.revoke({
          project_id: projectId,
          node_id: node.node_id,
          actor: node.actor,
        });
        setResult((previous) =>
          previous
            ? {
                ...previous,
                nodes: previous.nodes.map((entry) =>
                  entry.node_id === result.node_id
                    ? { ...entry, state: 'revoked', health: 'unknown' as const }
                    : entry,
                ),
              }
            : previous,
        );
        setDialog(null);
      } catch (error) {
        setDialog((current) =>
          current
            ? {
                ...current,
                resolving: false,
                error: error instanceof Error ? error.message : 'Falha ao revogar node',
              }
            : current,
        );
      }
    },
    [apiClient, dialog, projectId],
  );

  const renderEmpty = () => (
    <p className="node-list__empty">Nenhum node autenticado encontrado.</p>
  );

  const renderRow = (node: NodeStatus, isStale: boolean) => {
    const allowed = isAllowedToRevoke(node) && !isStale;
    return (
      <li key={node.node_id} role="listitem" className="node-list__row" data-state={node.state}>
        <button
          type="button"
          className="node-list__select"
          onClick={() => onSelect?.(node.node_id)}
        >
          <span className="node-list__name">{node.display_name}</span>
          <span className="node-list__meta">
            {node.node_id} · {node.peer_id} · {node.project_id}
          </span>
          <span className="node-list__state">
            {STATE_LABEL[node.state]} · {HEALTH_LABEL[node.health]}
          </span>
        </button>
        {allowed && node.health === 'healthy' ? (
          <button
            type="button"
            className="node-list__revoke"
            aria-label={`Revoke node ${node.node_id}`}
            onClick={() => setDialog({ node, resolving: false })}
          >
            Revogar
          </button>
        ) : null}
      </li>
    );
  };

  const renderDialog = () => {
    if (!dialog) return null;
    return (
      <div
        className="node-list__dialog-backdrop"
        role="presentation"
        onClick={() => {
          if (!dialog.resolving) setDialog(null);
        }}
      >
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="node-revoke-title"
          aria-describedby="node-revoke-body"
          className="node-list__dialog"
          onClick={(event) => event.stopPropagation()}
        >
          <h2 id="node-revoke-title">Revogar node?</h2>
          <p id="node-revoke-body">
            Esta ação encerra a sessão autenticada do node {dialog.node.node_id} e fecha
            streams remotos ativos. Não pode ser revertida.
          </p>
          {dialog.error ? (
            <p role="alert" className="node-list__error">
              {dialog.error}
            </p>
          ) : null}
          <div className="node-list__dialog-actions">
            <button
              type="button"
              onClick={() => setDialog(null)}
              disabled={dialog.resolving}
            >
              Cancelar
            </button>
            <button
              ref={confirmButtonRef}
              type="button"
              onClick={() => void handleRevoke(dialog.node)}
              disabled={dialog.resolving}
            >
              {dialog.resolving ? 'Revogando…' : 'Confirmar revogação'}
            </button>
          </div>
        </div>
      </div>
    );
  };

  const now = nowMs();
  const stale = result ? isStaleResponse(result, now) : false;

  return (
    <section className="node-list" aria-label="Lista de nodes autenticados">
      <header className="node-list__header">
        <h2>Nodes remotos</h2>
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
          aria-label="Atualizar lista de nodes"
        >
          {loading ? 'Atualizando…' : 'Atualizar'}
        </button>
      </header>
      {stale ? (
        <div role="alert" className="node-list__stale-banner">
          Resposta stale (mais de {Math.round(STALE_RESPONSE_THRESHOLD_MS / 1000)}s). Ações
          de revoke estão desabilitadas até a próxima atualização.
        </div>
      ) : null}
      {result && result.nodes.length === 0 ? renderEmpty() : null}
      {result && result.nodes.length > 0 ? (
        <ul role="list" className="node-list__items">
          {result.nodes.map((node) => renderRow(node, stale))}
        </ul>
      ) : null}
      {renderDialog()}
    </section>
  );
}
