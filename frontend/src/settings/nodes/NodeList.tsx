import { useEffect, useState, useCallback, useRef, useMemo } from 'react';
import {
  NodeStatus,
  NodeListResult,
  NodeManagementApiClient,
  NodeRevokeResult,
  isStaleResponse,
  STALE_RESPONSE_THRESHOLD_MS,
} from '../../api/node-management';

interface NodeListProps {
  projectId: string;
  apiClient: NodeManagementApiClient;
  onSelect?: (nodeId: string) => void;
  onError?: (error: string) => void;
  nowMs?: () => number;
}

interface RevokeDialogState {
  node: NodeStatus;
  fetchedAtMs: number;
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

const STALE_RECHECK_MS = 1_000;
const MAX_CAPABILITIES_DISPLAYED = 5;

function isAllowedToRevoke(node: NodeStatus): boolean {
  return node.state === 'active' || node.state === 'expired';
}

function effectiveHealth(node: NodeStatus, isStale: boolean): NodeStatus['health'] {
  return isStale ? 'stale' : node.health;
}

export function NodeList({
  projectId,
  apiClient,
  onSelect,
  onError,
  nowMs = () => Date.now(),
}: NodeListProps) {
  const [result, setResult] = useState<NodeListResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [dialog, setDialog] = useState<RevokeDialogState | null>(null);
  const [now, setNow] = useState(() => nowMs());
  const [staleDialogOpen, setStaleDialogOpen] = useState(false);
  const confirmButtonRef = useRef<HTMLButtonElement | null>(null);
  const cancelButtonRef = useRef<HTMLButtonElement | null>(null);
  const requestIdRef = useRef(0);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  const refresh = useCallback(async () => {
    const myId = requestIdRef.current + 1;
    requestIdRef.current = myId;
    setLoading(true);
    try {
      const next = await apiClient.list(projectId);
      // Drop responses that don't match the latest projectId request
      if (myId !== requestIdRef.current) return;
      setResult(next);
    } catch (error) {
      if (myId !== requestIdRef.current) return;
      onError?.(error instanceof Error ? error.message : 'Falha ao listar nodes');
    } finally {
      if (myId === requestIdRef.current) setLoading(false);
    }
  }, [apiClient, projectId, onError]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Periodically re-evaluate staleness using the latest `now`
  useEffect(() => {
    const interval = setInterval(() => setNow(nowMs()), STALE_RECHECK_MS);
    return () => clearInterval(interval);
  }, [nowMs]);

  const resultRef = useRef(result);
  resultRef.current = result;

  const stale = result ? isStaleResponse(result, now) : false;

  // When staleness flips on, close any open revoke dialog to prevent late
  // confirmation against an outdated list.
  useEffect(() => {
    if (stale && dialog && !dialog.resolving) {
      setDialog(null);
    }
  }, [stale, dialog]);

  // Focus management for the revoke dialog
  useEffect(() => {
    if (dialog && confirmButtonRef.current) {
      confirmButtonRef.current.focus();
    }
  }, [dialog]);

  // Focus trap + Esc handler (Esc ignored while a revoke is in flight)
  useEffect(() => {
    if (!dialog) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        if (dialog.resolving) return;
        setDialog(null);
        return;
      }
      if (event.key === 'Tab' && dialogRef.current) {
        const focusable = Array.from(
          dialogRef.current.querySelectorAll<HTMLElement>(
            'button:not([disabled]), [href], input:not([disabled])',
          ),
        );
        if (focusable.length === 0) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        const active = document.activeElement;
        if (event.shiftKey && active === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && active === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [dialog]);

  const handleRevoke = useCallback(
    async (state: RevokeDialogState) => {
      // Defensive freshness check: refuse to call the bridge if the
      // response that opened the dialog is now stale.
      const currentResult = resultRef.current;
      if (!currentResult) return;
      if (isStaleResponse(currentResult, nowMs())) {
        setStaleDialogOpen(true);
        setDialog(null);
        return;
      }
      setDialog((current) => (current ? { ...current, resolving: true, error: undefined } : current));
      try {
        const result: NodeRevokeResult = await apiClient.revoke({
          project_id: projectId,
          node_id: state.node.node_id,
          actor: state.node.actor,
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
    [apiClient, projectId, nowMs],
  );

  const openRevokeDialog = useCallback((node: NodeStatus) => {
    const r = resultRef.current;
    if (!r) return;
    setDialog({
      node,
      fetchedAtMs: r.fetched_at_ms,
      resolving: false,
    });
  }, []);

  const renderEmpty = () => (
    <p className="node-list__empty">Nenhum node autenticado encontrado.</p>
  );

  const renderRow = (node: NodeStatus, isStaleRow: boolean) => {
    const allowed = isAllowedToRevoke(node) && !isStaleRow;
    const health = effectiveHealth(node, isStaleRow);
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
            {STATE_LABEL[node.state]} · {HEALTH_LABEL[health]}
          </span>
          {node.capabilities.length > 0 ? (
            <span className="node-list__caps">
              {node.capabilities.slice(0, MAX_CAPABILITIES_DISPLAYED).join(', ')}
              {node.capabilities.length > MAX_CAPABILITIES_DISPLAYED
                ? ` (+${node.capabilities.length - MAX_CAPABILITIES_DISPLAYED})`
                : null}
            </span>
          ) : null}
        </button>
        {allowed && !isStaleRow && health === 'healthy' ? (
          <button
            type="button"
            className="node-list__revoke"
            aria-label={`Revoke node ${node.node_id}`}
            onClick={() => openRevokeDialog(node)}
          >
            Revogar
          </button>
        ) : null}
      </li>
    );
  };

  const renderDialog = () => {
    if (!dialog) return null;
    const titleId = 'node-revoke-title';
    const bodyId = 'node-revoke-body';
    return (
      <div
        className="node-list__dialog-backdrop"
        role="presentation"
        onClick={() => {
          if (!dialog.resolving) setDialog(null);
        }}
      >
        <div
          ref={dialogRef}
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          aria-describedby={bodyId}
          className="node-list__dialog"
          onClick={(event) => event.stopPropagation()}
        >
          <h2 id={titleId}>Revogar node?</h2>
          <p id={bodyId}>
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
              ref={cancelButtonRef}
              type="button"
              onClick={() => setDialog(null)}
              disabled={dialog.resolving}
            >
              Cancelar
            </button>
            <button
              ref={confirmButtonRef}
              type="button"
              onClick={() => void handleRevoke(dialog)}
              disabled={dialog.resolving}
            >
              {dialog.resolving ? 'Revogando…' : 'Confirmar revogação'}
            </button>
          </div>
        </div>
      </div>
    );
  };

  const renderStaleDialog = () => {
    if (!staleDialogOpen) return null;
    return (
      <div className="node-list__dialog-backdrop" role="presentation">
        <div
          role="alertdialog"
          aria-modal="true"
          aria-labelledby="node-stale-title"
          className="node-list__dialog"
        >
          <h2 id="node-stale-title">Lista desatualizada</h2>
          <p>
            A lista de nodes ultrapassou o limite de {Math.round(STALE_RESPONSE_THRESHOLD_MS / 1000)}s
            e a revogação não pode ser confirmada com dados potencialmente imprecisos.
            Atualize a lista e tente novamente.
          </p>
          <div className="node-list__dialog-actions">
            <button
              type="button"
              onClick={() => {
                setStaleDialogOpen(false);
                void refresh();
              }}
            >
              Atualizar agora
            </button>
            <button type="button" onClick={() => setStaleDialogOpen(false)}>
              Fechar
            </button>
          </div>
        </div>
      </div>
    );
  };

  const dialogStaleSince = useMemo(() => {
    if (!dialog || !result) return 0;
    return now - dialog.fetchedAtMs;
  }, [dialog, result, now]);
  const dialogWillBeStale = dialogStaleSince > STALE_RESPONSE_THRESHOLD_MS;

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
      {renderStaleDialog()}
      {dialog ? (
        <span className="node-list__sr-only" aria-live="polite">
          {dialogWillBeStale
            ? 'Aviso: a lista ultrapassou o limite de frescor. A confirmação será recusada.'
            : ''}
        </span>
      ) : null}
    </section>
  );
}
