import React, { useCallback, useEffect, useState } from 'react';
import { defaultSessionApi, SessionApiClient } from '../api/sessions';
import { CreateSessionInput, SessionSummary } from '../types/session';
import './SessionList.css';

export interface SessionListProps {
  projectId: string;
  agentId: string;
  agentName: string;
  apiClient?: SessionApiClient;
  pageSize?: number;
  onOpenSession?: (session: SessionSummary) => void;
}

export const SessionList: React.FC<SessionListProps> = ({
  projectId,
  agentId,
  agentName,
  apiClient = defaultSessionApi,
  pageSize = 10,
  onOpenSession,
}) => {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [total, setTotal] = useState<number>(0);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState<boolean>(false);
  const [createTitle, setCreateTitle] = useState<string>('');
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);

  const fetchSessions = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await apiClient.list({
        project_id: projectId,
        agent_id: agentId,
        limit: pageSize,
        offset: 0,
      });
      setSessions(response.sessions);
      setTotal(response.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Falha ao carregar conversas.');
    } finally {
      setIsLoading(false);
    }
  }, [agentId, apiClient, pageSize, projectId]);

  useEffect(() => {
    void fetchSessions();
  }, [fetchSessions]);

  const handleCreateSession = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (isSubmitting) return;

    const title = createTitle.trim();
    if (title.length > 256) {
      setError('O título da conversa deve ter no máximo 256 caracteres.');
      return;
    }

    setIsSubmitting(true);
    setError(null);
    const input: CreateSessionInput = {
      project_id: projectId,
      agent_id: agentId,
      title: title || null,
      correlation_id: `session_${Date.now().toString(36)}`,
    };

    try {
      await apiClient.create(input);
      setCreateTitle('');
      setShowCreateForm(false);
      await fetchSessions();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Falha ao criar conversa.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <section className="session-list-container" aria-label={`Conversas de ${agentName}`}>
      <header className="session-list-header">
        <div>
          <h3>Conversas de {agentName}</h3>
          <p className="session-list-subtitle">
            {total === 0 ? 'Nenhuma conversa iniciada' : `${total} conversa${total === 1 ? '' : 's'}`}
          </p>
        </div>
        <button
          type="button"
          className="session-list-create-button"
          onClick={() => {
            setShowCreateForm((current) => !current);
            setError(null);
          }}
          aria-label={showCreateForm
            ? 'Fechar formulário de nova conversa'
            : 'Abrir formulário de nova conversa'}
        >
          {showCreateForm ? 'Cancelar' : '+ Nova conversa'}
        </button>
      </header>

      {showCreateForm && (
        <form className="session-create-form" onSubmit={handleCreateSession} noValidate>
          <div className="session-create-field">
            <label htmlFor={`session-title-${agentId}`}>Título da conversa</label>
            <input
              id={`session-title-${agentId}`}
              type="text"
              value={createTitle}
              maxLength={256}
              disabled={isSubmitting}
              onChange={(event) => setCreateTitle(event.target.value)}
              placeholder="Opcional"
            />
          </div>
          <button type="submit" disabled={isSubmitting}>
            {isSubmitting ? 'Criando...' : 'Criar conversa'}
          </button>
        </form>
      )}

      {isLoading ? (
        <div className="session-list-state" role="status" aria-busy="true">
          Carregando conversas...
        </div>
      ) : error ? (
        <div className="session-list-state session-list-error" role="alert">
          <p>{error}</p>
          <button type="button" onClick={() => void fetchSessions()}>
            Tentar novamente
          </button>
        </div>
      ) : sessions.length === 0 ? (
        <div className="session-list-state">
          Nenhuma conversa iniciada para este agent.
        </div>
      ) : (
        <ul className="session-list" role="list">
          {sessions.map((session) => (
            <li className="session-card" key={session.id}>
              <div className="session-card-main">
                <strong>{session.title?.trim() || 'Conversa sem título'}</strong>
                <span className={`session-status session-status--${session.status}`}>
                  {session.status}
                </span>
                <span className="session-card-meta">
                  {session.message_count} mensagens · Atualizada em{' '}
                  {new Date(session.updated_at).toLocaleString()}
                </span>
              </div>
              {onOpenSession && (
                <button
                  type="button"
                  className="session-open-button"
                  onClick={() => onOpenSession(session)}
                >
                  Abrir conversa
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
};
