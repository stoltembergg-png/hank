import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  AgentInstructionSnapshot,
  defaultAgentInstructionApi,
} from '@/api/agent-instructions';
import { InstructionsFormState, InstructionsPageProps } from './types';
import './InstructionsPage.css';

const MAX_ALLOWED_BUDGET = 256 * 1024;

function validateSnapshot(snapshot: AgentInstructionSnapshot): string | null {
  if (snapshot.layer !== 'agent') return 'Snapshot inválido: apenas a camada Agent pode ser editada';
  if (!Number.isSafeInteger(snapshot.max_total_bytes)
    || snapshot.max_total_bytes <= 0
    || snapshot.max_total_bytes > MAX_ALLOWED_BUDGET) {
    return 'Budget de instruções inválido';
  }
  if (snapshot.content.length > snapshot.max_total_bytes) {
    return `Conteúdo excede o budget de ${snapshot.max_total_bytes} bytes`;
  }
  return null;
}

function mapError(error: unknown): string {
  const message = error instanceof Error ? error.message : 'Falha ao salvar instruções';
  const normalized = message.toLocaleLowerCase('en-US');
  if (normalized.includes('stale') || normalized.includes('version') || normalized.includes('concurrency')) {
    return 'As instruções foram modificadas por outro processo. Recarregue e tente novamente.';
  }
  if (normalized.includes('budget') || normalized.includes('oversized')) {
    return 'O conteúdo excede o budget de instruções e não foi truncado.';
  }
  return message;
}

export const InstructionsPage: React.FC<InstructionsPageProps> = ({
  projectId,
  agentId,
  onBack,
  onSaved,
  apiClient = defaultAgentInstructionApi,
}) => {
  const [loaded, setLoaded] = useState(false);
  const [snapshot, setSnapshot] = useState<AgentInstructionSnapshot | null>(null);
  const [state, setState] = useState<InstructionsFormState>({
    content: '',
    initialContent: '',
    maxTotalBytes: 0,
    expectedVersion: '',
    provenance: 'agent',
    isSubmitting: false,
    error: null,
  });
  const mountedRef = useRef(true);

  const fetchInstructions = useCallback(async () => {
    setLoaded(false);
    setState((previous) => ({ ...previous, error: null }));
    try {
      const fetched = await apiClient.get(projectId, agentId);
      if (!mountedRef.current) return;
      setLoaded(true);
      if (!fetched) {
        setSnapshot(null);
        return;
      }

      const validationError = validateSnapshot(fetched);
      if (validationError) {
        setSnapshot(null);
        setState((previous) => ({ ...previous, error: validationError }));
        return;
      }

      setSnapshot(fetched);
      setState((previous) => ({
        ...previous,
        content: fetched.content,
        initialContent: fetched.content,
        maxTotalBytes: fetched.max_total_bytes,
        expectedVersion: fetched.updated_at,
        provenance: fetched.provenance,
        error: null,
      }));
    } catch {
      if (!mountedRef.current) return;
      setLoaded(true);
      setState((previous) => ({ ...previous, error: 'Falha ao carregar instruções' }));
    }
  }, [agentId, apiClient, projectId]);

  useEffect(() => {
    mountedRef.current = true;
    void fetchInstructions();
    return () => {
      mountedRef.current = false;
    };
  }, [fetchInstructions]);

  const hasChanges = () => state.content !== state.initialContent;

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!snapshot) return;
    if (state.content.length > state.maxTotalBytes) {
      setState((previous) => ({
        ...previous,
        error: `Conteúdo excede o budget de ${state.maxTotalBytes} bytes`,
      }));
      return;
    }
    if (!hasChanges()) return;

    setState((previous) => ({ ...previous, isSubmitting: true, error: null }));
    try {
      const updated = await apiClient.update({
        project_id: projectId,
        agent_id: agentId,
        layer: 'agent',
        content: state.content,
        max_total_bytes: state.maxTotalBytes,
        expected_version: state.expectedVersion,
      });
      if (!mountedRef.current) return;
      onSaved?.(updated);
      onBack();
    } catch (error) {
      if (!mountedRef.current) return;
      setState((previous) => ({ ...previous, isSubmitting: false, error: mapError(error) }));
    }
  };

  const handleCancel = () => {
    if (!hasChanges()) {
      onBack();
      return;
    }
    if (window.confirm('Existem alterações não salvas. Deseja realmente sair?')) onBack();
  };

  if (!loaded) {
    return (
      <section className="instructions-page" aria-label="Instruções do Agent">
        <header className="instructions-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Instruções do Agent</h1>
        </header>
        <div className="instructions-page__loading" role="status" aria-live="polite">
          Carregando instruções...
        </div>
      </section>
    );
  }

  if (!snapshot) {
    return (
      <section className="instructions-page" aria-label="Instruções do Agent">
        <header className="instructions-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Instruções do Agent</h1>
        </header>
        {state.error ? (
          <div className="instructions-page__error" role="alert">
            <p>{state.error}</p>
            <button type="button" onClick={() => void fetchInstructions()}>Tentar novamente</button>
          </div>
        ) : (
          <div className="instructions-page__unsupported" role="status">
            <strong>Nenhum serviço de instruções disponível para este agent.</strong>
            <span>A camada Agent permanece sem conteúdo editável neste momento.</span>
          </div>
        )}
      </section>
    );
  }

  return (
    <section className="instructions-page" aria-label="Instruções do Agent">
      <header className="instructions-page__header">
        <button type="button" onClick={handleCancel} aria-label="Voltar" disabled={state.isSubmitting}>
          ← Voltar
        </button>
        <h1>Instruções do Agent</h1>
      </header>

      {state.error && <div className="instructions-page__error" role="alert"><p>{state.error}</p></div>}

      <div className="instructions-page__boundary" role="note">
        <strong>Camada: Agent</strong>
        <span>Proveniência: {state.provenance}</span>
        <span>Esta camada não substitui as políticas de segurança e sistema.</span>
      </div>

      <form className="instructions-page__form" aria-label="Formulário de instruções Agent" noValidate onSubmit={handleSubmit}>
        <div className="instructions-page__field">
          <label htmlFor="agent-instruction-content">Conteúdo da camada Agent</label>
          <textarea
            id="agent-instruction-content"
            value={state.content}
            onChange={(event) => setState((previous) => ({ ...previous, content: event.target.value, error: null }))}
            maxLength={state.maxTotalBytes}
            rows={12}
            disabled={state.isSubmitting}
            aria-describedby="agent-instruction-hint agent-instruction-count"
          />
          <div className="instructions-page__hint" id="agent-instruction-hint">
            Texto não confiável: será tratado como conteúdo da camada Agent, não como comando do sistema.
            <span id="agent-instruction-count" aria-live="polite">
              {state.content.length}/{state.maxTotalBytes} bytes
            </span>
          </div>
        </div>

        <div className="instructions-page__budget" role="status">
          Budget: {state.maxTotalBytes} bytes · Proveniência: {state.provenance}
        </div>

        <section className="instructions-page__preview" aria-label="Prévia plain-text">
          <h2>Prévia plain-text</h2>
          <pre data-testid="instruction-preview">{state.content}</pre>
        </section>

        <div className="instructions-page__actions">
          <button type="button" onClick={handleCancel} disabled={state.isSubmitting}>Cancelar</button>
          <button type="submit" disabled={state.isSubmitting || !hasChanges()}>
            {state.isSubmitting ? 'Salvando instruções...' : 'Salvar instruções'}
          </button>
        </div>
      </form>
    </section>
  );
};