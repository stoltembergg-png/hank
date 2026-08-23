import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { defaultMemoryApi, MemoryApiClient } from '../api/memory';
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
  apiClient?: MemoryApiClient;
}

export const MemoryPanel: React.FC<MemoryPanelProps> = ({
  projectId,
  apiClient = defaultMemoryApi,
}) => {
  const [memories, setMemories] = useState<MemorySummary[]>([]);
  const [status, setStatus] = useState<MemoryStatus | ''>('');
  const [memoryType, setMemoryType] = useState<MemoryType | ''>('');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
            </li>
          ))}
        </ul>
      )}
    </section>
  );
};
