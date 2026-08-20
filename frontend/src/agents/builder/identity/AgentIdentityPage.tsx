import React, { useState, useEffect, useCallback, useRef } from 'react';
import { defaultAgentApi } from '@/api/agents';
import { AgentSummary, UpdateAgentInput, UpdateAgentOutput } from '@/types/agent';
import { AgentIdentityPageProps, AgentIdentityFormState, AgentIdentityFormData } from './types';
import './AgentIdentityPage.css';

const MAX_NAME_LENGTH = 120;
const MAX_DESCRIPTION_LENGTH = 500;

export const AgentIdentityPage: React.FC<AgentIdentityPageProps> = ({
  projectId,
  agentId,
  onBack,
  onSaved,
  apiClient = defaultAgentApi,
}) => {
  const [state, setState] = useState<AgentIdentityFormState>({
    name: '',
    description: '',
    isSubmitting: false,
    error: null,
    expectedVersion: '',
    initialData: { name: '', description: '' },
  });
  const [agent, setAgent] = useState<AgentSummary | null>(null);
  const isMountedRef = useRef(true);

  const fetchAgent = useCallback(async () => {
    try {
      const fetched = await apiClient.get(projectId, agentId);
      if (!isMountedRef.current) return;

      if (!fetched) {
        setState((prev) => ({
          ...prev,
          error: 'Agent não encontrado',
        }));
        return;
      }

      setAgent(fetched);
      const initialData: AgentIdentityFormData = {
        name: fetched.name,
        description: fetched.description || '',
      };

      setState((prev) => ({
        ...prev,
        name: initialData.name,
        description: initialData.description,
        expectedVersion: fetched.updated_at,
        initialData,
      }));
    } catch (err) {
      if (!isMountedRef.current) return;
      setState((prev) => ({
        ...prev,
        error: err instanceof Error ? err.message : 'Falha ao carregar agent',
      }));
    }
  }, [apiClient, projectId, agentId]);

  useEffect(() => {
    isMountedRef.current = true;
    fetchAgent();
    return () => {
      isMountedRef.current = false;
    };
  }, [fetchAgent]);

  const validateForm = (): string | null => {
    const name = state.name.trim();
    if (!name) return 'Nome é obrigatório';
    if (name.length > MAX_NAME_LENGTH) return `Nome deve ter no máximo ${MAX_NAME_LENGTH} caracteres`;
    if (state.description.length > MAX_DESCRIPTION_LENGTH) {
      return `Descrição deve ter no máximo ${MAX_DESCRIPTION_LENGTH} caracteres`;
    }
    return null;
  };

  const hasChanges = (): boolean => {
    return (
      state.name.trim() !== state.initialData.name ||
      state.description.trim() !== state.initialData.description
    );
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const validationError = validateForm();
    if (validationError) {
      setState((prev) => ({ ...prev, error: validationError }));
      return;
    }

    if (!hasChanges()) {
      onBack();
      return;
    }

    setState((prev) => ({ ...prev, isSubmitting: true, error: null }));

    try {
      const input: UpdateAgentInput = {
        project_id: projectId,
        agent_id: agentId,
        name: state.name.trim(),
        description: state.description.trim() || null,
        expected_version: state.expectedVersion,
      };

      const output: UpdateAgentOutput = await apiClient.update(input);

      if (!isMountedRef.current) return;

      if (onSaved) {
        onSaved(output.agent);
      }
      onBack();
    } catch (err) {
      if (!isMountedRef.current) return;

      const message = err instanceof Error ? err.message : 'Falha ao salvar';
      let userMessage = message;

      if (message.includes('stale') || message.includes('version') || message.includes('concurrency')) {
        userMessage = 'O agent foi modificado por outro processo. Recarregue e tente novamente.';
      } else if (message.includes('archived') || message.includes('inactive')) {
        userMessage = 'Não é possível editar um agent arquivado ou inativo.';
      } else if (message.includes('forbidden') || message.includes('permission')) {
        userMessage = 'Você não tem permissão para editar este agent.';
      }

      setState((prev) => ({
        ...prev,
        isSubmitting: false,
        error: userMessage,
      }));
    }
  };

  const handleCancel = () => {
    if (hasChanges()) {
      if (window.confirm('Existem alterações não salvas. Deseja realmente sair?')) {
        onBack();
      }
    } else {
      onBack();
    }
  };

  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setState((prev) => ({
      ...prev,
      name: e.target.value,
      error: null,
    }));
  };

  const handleDescriptionChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setState((prev) => ({
      ...prev,
      description: e.target.value,
      error: null,
    }));
  };

  const nameLength = state.name.trim().length;
  const descriptionLength = state.description.length;

  if (!agent) {
    if (state.error) {
      return (
        <section className="agent-identity-container" aria-label="Identidade do Agent">
          <header className="agent-identity-header">
            <button type="button" className="back-button" onClick={onBack} aria-label="Voltar">
              ← Voltar
            </button>
            <h2>Identidade do Agent</h2>
          </header>
          <div className="agent-identity-error" role="alert">
            <p>{state.error}</p>
            <button type="button" onClick={fetchAgent}>
              Tentar novamente
            </button>
          </div>
        </section>
      );
    }

    return (
      <section className="agent-identity-container" aria-label="Identidade do Agent">
        <header className="agent-identity-header">
          <button type="button" className="back-button" onClick={onBack} aria-label="Voltar">
            ← Voltar
          </button>
          <h2>Identidade do Agent</h2>
        </header>
        <div className="agent-identity-loading" role="status" aria-live="polite">
          Carregando agent...
        </div>
      </section>
    );
  }

  const isArchived = agent.status === 'inactive' || agent.status === 'suspended';

  return (
    <section className="agent-identity-container" aria-label="Identidade do Agent">
      <header className="agent-identity-header">
        <button type="button" className="back-button" onClick={handleCancel} aria-label="Voltar">
          ← Voltar
        </button>
        <h2>Identidade do Agent</h2>
      </header>

      {state.error && (
        <div className="agent-identity-error" role="alert">
          <p>{state.error}</p>
        </div>
      )}

      <form onSubmit={handleSubmit} className="agent-identity-form" noValidate data-testid="agent-identity-form">
        <div className="form-field">
          <label htmlFor="agent-name">Nome <span className="required" aria-hidden="true">*</span></label>
          <input
            id="agent-name"
            type="text"
            value={state.name}
            onChange={handleNameChange}
            maxLength={MAX_NAME_LENGTH}
            disabled={state.isSubmitting || isArchived}
            required
            aria-describedby="name-hint name-count"
            autoFocus
          />
          <div className="field-hints">
            <span id="name-hint">Nome único do agent dentro do projeto</span>
            <span id="name-count" className="char-count" aria-live="polite" data-testid="name-char-count">
              {nameLength}/{MAX_NAME_LENGTH}
            </span>
          </div>
        </div>

        <div className="form-field">
          <label htmlFor="agent-description">Descrição</label>
          <textarea
            id="agent-description"
            value={state.description}
            onChange={handleDescriptionChange}
            maxLength={MAX_DESCRIPTION_LENGTH}
            rows={4}
            disabled={state.isSubmitting || isArchived}
            aria-describedby="desc-hint desc-count"
          />
          <div className="field-hints">
            <span id="desc-hint">Descrição opcional do propósito do agent</span>
            <span id="desc-count" className="char-count" aria-live="polite" data-testid="desc-char-count">
              {descriptionLength}/{MAX_DESCRIPTION_LENGTH}
            </span>
          </div>
        </div>

        <div className="agent-meta">
          <div className="meta-item">
            <span className="meta-label">Status</span>
            <span className={`meta-value agent-status agent-status--${agent.status}`}>
              {agent.status}
            </span>
          </div>
          <div className="meta-item">
            <span className="meta-label">ID</span>
            <span className="meta-value mono">{agent.id}</span>
          </div>
          <div className="meta-item">
            <span className="meta-label">Projeto</span>
            <span className="meta-value mono">{projectId}</span>
          </div>
          <div className="meta-item">
            <span className="meta-label">Última atualização</span>
            <span className="meta-value">{new Date(agent.updated_at).toLocaleString()}</span>
          </div>
        </div>

        {isArchived && (
          <div className="archived-notice" role="status">
            Este agent está arquivado e não pode ser editado.
          </div>
        )}

        <div className="form-actions">
          <button
            type="button"
            className="btn btn-secondary"
            onClick={handleCancel}
            disabled={state.isSubmitting}
          >
            Cancelar
          </button>
          <button
            type="submit"
            className="btn btn-primary"
            disabled={state.isSubmitting || !hasChanges() || isArchived}
          >
            {state.isSubmitting ? 'Salvando...' : 'Salvar alterações'}
          </button>
        </div>
      </form>
    </section>
  );
};