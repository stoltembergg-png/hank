import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { defaultMemoryApi, MemoryApiClient, MemoryMutationEdit } from '../api/memory';
import { MemoryStatus, MemorySummary, MemoryType } from '../types/memory';
import './MemoryPanel.css';

const MAX_PREVIEW_CHARS = 320;

function safePreview(content: string): string {
  const redacted = content.replace(
    /(api[_ -]?key|authorization|password|secret|bearer)\s*[:=]\s*[^\s,;]+/gi,
    '$1: [REDACTED]',
  );
  return redacted.length > MAX_PREVIEW_CHARS
    ? `${redacted.slice(0, MAX_PREVIEW_CHARS)}…`
    : redacted;
}

function statusLabel(status: MemoryStatus): string {
  return status === 'candidate' ? 'Não ativo (candidate)' : status;
}

export interface MemoryPanelProps {
  projectId: string;
  actorId?: string;
  apiClient?: MemoryApiClient;
}

export const MemoryPanel: React.FC<MemoryPanelProps> = ({
  projectId,
  actorId = 'desktop-operator',
  apiClient = defaultMemoryApi,
}) => {
  const [memories, setMemories] = useState<MemorySummary[]>([]);
  const [status, setStatus] = useState<MemoryStatus | ''>('');
  const [memoryType, setMemoryType] = useState<MemoryType | ''>('');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [mutationError, setMutationError] = useState<string | null>(null);
  const [editingMemoryId, setEditingMemoryId] = useState<string | null>(null);
  const [draftContent, setDraftContent] = useState('');
  const [draftSummary, setDraftSummary] = useState('');
  const [draftImportance, setDraftImportance] = useState('0.5');
  const [mutatingMemoryId, setMutatingMemoryId] = useState<string | null>(null);

  const fetchMemories = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const result = await apiClient.list({
        project_id: projectId,
        status: status || undefined,
        memory_type: memoryType || undefined,
        limit: 100,
      });
      if (result.project_id !== projectId) {
        throw new Error('Resposta de memória fora do projeto selecionado.');
      }
      setMemories(result.memories.filter((memory) => memory.project_id === projectId));
    } catch (err) {
      setMemories([]);
      setError(err instanceof Error ? err.message : 'Falha ao carregar memórias.');
    } finally {
      setIsLoading(false);
    }
  }, [apiClient, memoryType, projectId, status]);

  useEffect(() => {
    void fetchMemories();
  }, [fetchMemories]);

  const visibleMemories = useMemo(
    () => memories.filter((memory) => !status || memory.status === status)
      .filter((memory) => !memoryType || memory.memory_type === memoryType),
    [memoryType, memories, status],
  );

  const startEditing = (memory: MemorySummary) => {
    setEditingMemoryId(memory.id);
    setDraftContent(memory.content);
    setDraftSummary(memory.summary ?? '');
    setDraftImportance(String(memory.importance));
    setMutationError(null);
  };

  const dispatchMutation = async (
    memory: MemorySummary,
    edit: MemoryMutationEdit,
    confirmationMessage: string,
  ) => {
    if (!apiClient.mutate) {
      setMutationError('Serviço de edição de memória indisponível.');
      return;
    }
    if (!window.confirm(confirmationMessage)) return;

    setMutationError(null);
    setMutatingMemoryId(memory.id);
    try {
      const updated = await apiClient.mutate({
        project_id: projectId,
        agent_id: memory.agent_id ?? undefined,
        memory_id: memory.id,
        actor_id: actorId,
        trace_id: memory.trace_id ?? createBoundedId('memory-trace'),
        operation_id: createBoundedId('memory-operation'),
        capability: 'memory.write',
        expected_version: memory.version ?? 1,
        confirmed: true,
        edit,
      });
      if (updated.project_id !== projectId || updated.id !== memory.id) {
        throw new Error('Resposta de memória fora do projeto selecionado.');
      }
      setEditingMemoryId(null);
      await fetchMemories();
    } catch (err) {
      setMutationError(memoryMutationError(err));
    } finally {
      setMutatingMemoryId(null);
    }
  };

  return (
    <section className="memory-panel" aria-label="Memórias do projeto">
      <header className="memory-panel-header">
        <div>
          <h3>Memórias do projeto</h3>
          <p className="memory-panel-scope">Projeto: <code>{projectId}</code></p>
        </div>
        <button type="button" onClick={() => void fetchMemories()} disabled={isLoading}>
          Atualizar
        </button>
      </header>

      <div className="memory-panel-filters" aria-label="Filtros de memória">
        <label>
          Filtrar por status
          <select value={status} onChange={(event) => setStatus(event.target.value as MemoryStatus | '')}>
            <option value="">Todos</option>
            <option value="candidate">Candidate</option>
            <option value="approved">Approved</option>
            <option value="rejected">Rejected</option>
            <option value="archived">Archived</option>
          </select>
        </label>
        <label>
          Filtrar por tipo
          <select value={memoryType} onChange={(event) => setMemoryType(event.target.value as MemoryType | '')}>
            <option value="">Todos</option>
            <option value="fact">Fact</option>
            <option value="preference">Preference</option>
            <option value="decision">Decision</option>
            <option value="lesson">Lesson</option>
            <option value="project_context">Project context</option>
            <option value="technical_context">Technical context</option>
            <option value="failure">Failure</option>
            <option value="successful_pattern">Successful pattern</option>
          </select>
        </label>
      </div>

      {isLoading && <p role="status" aria-busy="true">Carregando memórias...</p>}
      {!isLoading && error && <p role="alert">{error}</p>}
      {mutationError && <p className="memory-mutation-error" role="alert">{mutationError}</p>}
      {!isLoading && !error && visibleMemories.length === 0 && (
        <p role="status">Nenhuma memória encontrada neste projeto.</p>
      )}
      {!isLoading && !error && visibleMemories.length > 0 && (
        <ul className="memory-list" role="list">
          {visibleMemories.map((memory) => (
            <li className="memory-card" key={memory.id}>
              <div className="memory-card-header">
                <strong>{memory.memory_type}</strong>
                <span className={`memory-status ${memory.status}`}>{statusLabel(memory.status)}</span>
              </div>
              <p className="memory-content">{safePreview(memory.content)}</p>
              <dl className="memory-metadata">
                <dt>Provenance</dt><dd>{memory.provenance}</dd>
                <dt>Confidence</dt><dd>{memory.confidence.toFixed(2)}</dd>
                <dt>Importance</dt><dd>{memory.importance.toFixed(2)}</dd>
                <dt>Trace</dt><dd>{memory.trace_id ?? 'não disponível'}</dd>
              </dl>
              {editingMemoryId === memory.id ? (
                <div className="memory-edit-form" aria-label="Edição de memória">
                  <label>
                    Conteúdo
                    <textarea value={draftContent} onChange={(event) => setDraftContent(event.target.value)} />
                  </label>
                  <label>
                    Resumo
                    <input value={draftSummary} onChange={(event) => setDraftSummary(event.target.value)} />
                  </label>
                  <label>
                    Importância
                    <input
                      type="number"
                      min="0"
                      max="1"
                      step="0.1"
                      value={draftImportance}
                      onChange={(event) => setDraftImportance(event.target.value)}
                    />
                  </label>
                  <div className="memory-card-actions">
                    <button
                      type="button"
                      disabled={mutatingMemoryId === memory.id}
                      onClick={() => void dispatchMutation(
                        memory,
                        {
                          kind: 'update',
                          content: draftContent,
                          summary: draftSummary.trim() || null,
                          importance: Number(draftImportance),
                        },
                        'Confirmar edição desta memória?',
                      )}
                    >
                      Salvar edição
                    </button>
                    <button type="button" onClick={() => setEditingMemoryId(null)}>
                      Cancelar
                    </button>
                  </div>
                </div>
              ) : (
                <div className="memory-card-actions">
                  <button type="button" onClick={() => startEditing(memory)}>
                    Editar
                  </button>
                  {memory.status === 'candidate' && (
                    <>
                      <button
                        type="button"
                        disabled={mutatingMemoryId === memory.id}
                        onClick={() => void dispatchMutation(memory, { kind: 'approve' }, 'Confirmar aprovação desta memória?')}
                      >
                        Aprovar
                      </button>
                      <button
                        type="button"
                        disabled={mutatingMemoryId === memory.id}
                        onClick={() => void dispatchMutation(memory, { kind: 'reject' }, 'Confirmar rejeição desta memória?')}
                      >
                        Rejeitar
                      </button>
                    </>
                  )}
                  {memory.status === 'approved' && (
                    <button
                      type="button"
                      disabled={mutatingMemoryId === memory.id}
                      onClick={() => void dispatchMutation(memory, { kind: 'archive' }, 'Confirmar arquivamento desta memória?')}
                    >
                      Arquivar
                    </button>
                  )}
                  {memory.status === 'archived' && (
                    <button
                      type="button"
                      disabled={mutatingMemoryId === memory.id}
                      onClick={() => void dispatchMutation(memory, { kind: 'restore' }, 'Confirmar restauração desta memória?')}
                    >
                      Restaurar
                    </button>
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
};

function createBoundedId(prefix: string): string {
  const random = typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
  return `${prefix}-${random}`.slice(0, 128);
}

function memoryMutationError(error: unknown): string {
  const message = error instanceof Error ? error.message : '';
  if (/concurrency|stale|version/i.test(message)) {
    return 'Conflito de versão: a memória mudou antes da confirmação. Recarregue e tente novamente.';
  }
  return 'A mutation da memória foi rejeitada sem alterar o registro.';
}
