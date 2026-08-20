import React, { useState, useEffect, useCallback } from 'react';
import { AgentApiClient, defaultAgentApi } from '../api/agents';
import { AgentSummary, AgentStatus, ListAgentsInput } from '../types/agent';
import './AgentList.css';

export interface AgentListProps {
  projectId: string;
  apiClient?: AgentApiClient;
  _statusFilter?: AgentStatus;
  pageSize?: number;
}

export const AgentList: React.FC<AgentListProps> = ({
  projectId,
  apiClient = defaultAgentApi,
  pageSize = 10,
}) => {
  const [agents, setAgents] = useState<AgentSummary[]>([]);
  const [total, setTotal] = useState<number>(0);
  const [offset, setOffset] = useState<number>(0);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAgents = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const input: ListAgentsInput = {
        project_id: projectId,
        limit: pageSize,
        offset,
      };
      const response = await apiClient.list(input);
      setAgents(response.agents);
      setTotal(response.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Falha ao carregar agents');
    } finally {
      setIsLoading(false);
    }
  }, [apiClient, projectId, pageSize, offset]);

  useEffect(() => {
    fetchAgents();
  }, [fetchAgents]);

  const handleNextPage = () => {
    if (offset + pageSize < total) {
      setOffset((prev) => prev + pageSize);
    }
  };

  const handlePrevPage = () => {
    setOffset((prev) => Math.max(0, prev - pageSize));
  };

  const currentPage = Math.floor(offset / pageSize) + 1;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  if (isLoading) {
    return (
      <section className="agent-list-container" aria-label="Gerenciamento de Agents">
        <header className="agent-list-header">
          <h2>Agents</h2>
        </header>
        <div className="agent-list-loading" role="status" aria-live="polite">
          Carregando agents...
        </div>
      </section>
    );
  }

  if (error) {
    return (
      <section className="agent-list-container" aria-label="Gerenciamento de Agents">
        <header className="agent-list-header">
          <h2>Agents</h2>
        </header>
        <div className="agent-list-error" role="alert">
          <p>{error}</p>
          <button type="button" onClick={fetchAgents}>
            Tentar novamente
          </button>
        </div>
      </section>
    );
  }

  if (agents.length === 0) {
    return (
      <section className="agent-list-container" aria-label="Gerenciamento de Agents">
        <header className="agent-list-header">
          <h2>Agents</h2>
        </header>
        <div className="agent-list-empty">
          <p>Nenhum agent encontrado para este projeto.</p>
        </div>
      </section>
    );
  }

  return (
    <section className="agent-list-container" aria-label="Gerenciamento de Agents">
      <header className="agent-list-header">
        <h2>Agents ({total})</h2>
      </header>
      <table className="agent-list-table">
        <thead>
          <tr>
            <th scope="col">Nome</th>
            <th scope="col">Status</th>
            <th scope="col">Personality</th>
            <th scope="col">Criado em</th>
            <th scope="col">Atualizado em</th>
          </tr>
        </thead>
        <tbody>
          {agents.map((agent) => (
            <tr key={agent.id}>
              <td>{agent.name}</td>
              <td>
                <span className={`agent-status agent-status--${agent.status}`}>
                  {agent.status}
                </span>
              </td>
              <td>{agent.personality.name}</td>
              <td>{new Date(agent.created_at).toLocaleString()}</td>
              <td>{new Date(agent.updated_at).toLocaleString()}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {totalPages > 1 && (
        <nav className="agent-list-pagination" aria-label="Paginação de agents">
          <button
            type="button"
            onClick={handlePrevPage}
            disabled={currentPage === 1}
            aria-label="Página anterior"
          >
            Anterior
          </button>
          <span className="agent-list-page-info" aria-live="polite">
            Página {currentPage} de {totalPages}
          </span>
          <button
            type="button"
            onClick={handleNextPage}
            disabled={currentPage === totalPages}
            aria-label="Próxima página"
          >
            Próxima
          </button>
        </nav>
      )}
    </section>
  );
};