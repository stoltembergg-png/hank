import { useEffect, useState } from 'react';
import {
  NodeStatus,
  NodeManagementApiClient,
  defaultNodeManagementApi,
} from '../../api/node-management';

interface NodeDetailProps {
  projectId: string;
  nodeId: string | null;
  apiClient?: NodeManagementApiClient;
  onClose?: () => void;
}

function formatTimestamp(ms: number | null): string {
  if (!ms || ms <= 0) return '—';
  return new Date(ms).toISOString();
}

function describeHealth(health: NodeStatus['health']): string {
  switch (health) {
    case 'healthy':
      return 'Saudável: heartbeats dentro da janela esperada.';
    case 'stale':
      return 'Desatualizado: última resposta além do limite.';
    case 'unreachable':
      return 'Inacessível: o node não responde.';
    default:
      return 'Estado de saúde desconhecido.';
  }
}

export function NodeDetail({
  projectId,
  nodeId,
  apiClient = defaultNodeManagementApi,
  onClose,
}: NodeDetailProps) {
  const [node, setNode] = useState<NodeStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!nodeId) {
      setNode(null);
      setError(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    apiClient
      .get(projectId, nodeId)
      .then((next) => {
        if (!cancelled) setNode(next);
      })
      .catch((caught: unknown) => {
        if (!cancelled) {
          setError(caught instanceof Error ? caught.message : 'Falha ao obter node');
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [apiClient, nodeId, projectId]);

  if (!nodeId) {
    return (
      <section className="node-detail" aria-label="Detalhe de node">
        <p className="node-detail__empty">Selecione um node para inspecionar.</p>
      </section>
    );
  }

  if (loading) {
    return (
      <section className="node-detail" aria-label="Detalhe de node" aria-busy="true">
        <p>Carregando…</p>
      </section>
    );
  }

  if (error) {
    return (
      <section className="node-detail" aria-label="Detalhe de node">
        <p role="alert" className="node-detail__error">
          {error}
        </p>
        {onClose ? (
          <button type="button" onClick={onClose}>
            Fechar
          </button>
        ) : null}
      </section>
    );
  }

  if (!node) {
    return (
      <section className="node-detail" aria-label="Detalhe de node">
        <p>Node não encontrado.</p>
      </section>
    );
  }

  return (
    <section className="node-detail" aria-label="Detalhe de node">
      <header className="node-detail__header">
        <h2>{node.display_name}</h2>
        {onClose ? (
          <button type="button" onClick={onClose} aria-label="Fechar detalhe de node">
            Fechar
          </button>
        ) : null}
      </header>
      <dl className="node-detail__list">
        <dt>Node ID</dt>
        <dd>{node.node_id}</dd>
        <dt>Peer</dt>
        <dd>{node.peer_id}</dd>
        <dt>Project</dt>
        <dd>{node.project_id}</dd>
        <dt>Actor</dt>
        <dd>{node.actor}</dd>
        <dt>Protocolo</dt>
        <dd>{node.protocol_revision}</dd>
        <dt>State</dt>
        <dd>{node.state}</dd>
        <dt>Saúde</dt>
        <dd title={describeHealth(node.health)}>{node.health}</dd>
        <dt>Autenticado em</dt>
        <dd>{formatTimestamp(node.authenticated_at_ms)}</dd>
        <dt>Visto por último</dt>
        <dd>{formatTimestamp(node.last_seen_ms)}</dd>
        <dt>Stale desde</dt>
        <dd>{formatTimestamp(node.stale_since_ms)}</dd>
        <dt>Capabilities</dt>
        <dd>{node.capabilities.length === 0 ? '—' : node.capabilities.join(', ')}</dd>
      </dl>
      <p className="node-detail__note">
        Material de credencial nunca é exibido aqui. Apenas identificadores opacos,
        estado de saúde e capabilities.
      </p>
    </section>
  );
}
