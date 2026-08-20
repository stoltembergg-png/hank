import React, { useCallback, useEffect, useRef, useState } from 'react';
import { defaultAgentApi } from '@/api/agents';
import { AgentSummary, Personality } from '@/types/agent';
import {
  PersonalityFormData,
  PersonalityFormState,
  PersonalityPageProps,
} from './types';
import './PersonalityPage.css';

const MAX_NAME_LENGTH = 120;
const MAX_DESCRIPTION_LENGTH = 4_000;
const MAX_TRAITS = 32;
const MAX_TRAIT_LENGTH = 80;

const FORBIDDEN_MARKERS = [
  'api_key',
  'authorization:',
  'password',
  'ignore previous instructions',
];

const EMPTY_PERSONALITY: PersonalityFormData = {
  name: '',
  description: '',
  traits: [],
  communication_style: 'technical',
};

function normalizePersonality(personality: Personality): PersonalityFormData {
  return {
    name: personality.name,
    description: personality.description ?? '',
    traits: personality.traits.map((trait) => trait.trim()),
    communication_style: personality.communication_style,
  };
}

function containsForbiddenContent(value: string): boolean {
  const normalized = value.toLocaleLowerCase('en-US');
  return FORBIDDEN_MARKERS.some((marker) => normalized.includes(marker));
}

function parseTraits(value: string): string[] {
  if (value.trim() === '') return [];
  return value.split(',').map((trait) => trait.trim());
}

function samePersonality(left: PersonalityFormData, right: PersonalityFormData): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function mapServiceError(error: unknown): string {
  const message = error instanceof Error ? error.message : 'Falha ao salvar personalidade';
  const normalized = message.toLocaleLowerCase('en-US');

  if (normalized.includes('stale') || normalized.includes('version') || normalized.includes('concurrency')) {
    return 'A personalidade foi modificada por outro processo. Recarregue e tente novamente.';
  }
  if (normalized.includes('archived') || normalized.includes('inactive')) {
    return 'Este agent está arquivado ou inativo e não pode ter a personalidade editada.';
  }
  if (normalized.includes('forbidden') || normalized.includes('permission')) {
    return 'Você não tem permissão para editar a personalidade deste agent.';
  }
  return message;
}

export const PersonalityPage: React.FC<PersonalityPageProps> = ({
  projectId,
  agentId,
  onBack,
  onSaved,
  apiClient = defaultAgentApi,
}) => {
  const [agent, setAgent] = useState<AgentSummary | null>(null);
  const [state, setState] = useState<PersonalityFormState>({
    name: EMPTY_PERSONALITY.name,
    description: EMPTY_PERSONALITY.description ?? '',
    traits: EMPTY_PERSONALITY.traits,
    communicationStyle: EMPTY_PERSONALITY.communication_style,
    expectedVersion: '',
    initialData: EMPTY_PERSONALITY,
    isSubmitting: false,
    error: null,
  });
  const mountedRef = useRef(true);

  const fetchAgent = useCallback(async () => {
    try {
      const fetched = await apiClient.get(projectId, agentId);
      if (!mountedRef.current) return;

      if (!fetched) {
        setState((previous) => ({
          ...previous,
          error: 'Agent não encontrado',
        }));
        return;
      }

      const personality = normalizePersonality(fetched.personality);
      setAgent(fetched);
      setState((previous) => ({
        ...previous,
        name: personality.name,
        description: personality.description ?? '',
        traits: personality.traits,
        communicationStyle: personality.communication_style,
        initialData: personality,
        expectedVersion: fetched.updated_at,
        error: null,
      }));
    } catch {
      if (!mountedRef.current) return;
      setState((previous) => ({
        ...previous,
        error: 'Falha ao carregar personalidade',
      }));
    }
  }, [agentId, apiClient, projectId]);

  useEffect(() => {
    mountedRef.current = true;
    void fetchAgent();
    return () => {
      mountedRef.current = false;
    };
  }, [fetchAgent]);

  const currentPersonality = (): PersonalityFormData => ({
    name: state.name.trim(),
    description: state.description.trim(),
    traits: state.traits,
    communication_style: state.communicationStyle,
  });

  const hasChanges = (): boolean => !samePersonality(currentPersonality(), state.initialData);

  const validate = (): string | null => {
    const name = state.name.trim();
    const description = state.description.trim();

    if (!name) return 'Nome da personalidade é obrigatório';
    if (name.length > MAX_NAME_LENGTH) {
      return `Nome da personalidade deve ter no máximo ${MAX_NAME_LENGTH} caracteres`;
    }
    if (description.length > MAX_DESCRIPTION_LENGTH) {
      return `Descrição deve ter no máximo ${MAX_DESCRIPTION_LENGTH} caracteres`;
    }
    if (state.traits.length > MAX_TRAITS) {
      return `Informe no máximo ${MAX_TRAITS} traços`;
    }
    if (state.traits.some((trait) => trait.length === 0)) {
      return 'Traços não podem ser vazios';
    }
    if (state.traits.some((trait) => trait.length > MAX_TRAIT_LENGTH)) {
      return `Cada traço deve ter no máximo ${MAX_TRAIT_LENGTH} caracteres`;
    }
    if ([name, description, ...state.traits].some(containsForbiddenContent)) {
      return 'Conteúdo não permitido: instruções de sistema, credenciais e segredos não pertencem à personalidade';
    }
    return null;
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const validationError = validate();
    if (validationError) {
      setState((previous) => ({ ...previous, error: validationError }));
      return;
    }
    if (!hasChanges()) return;

    const personality = currentPersonality();
    setState((previous) => ({ ...previous, isSubmitting: true, error: null }));

    try {
      const output = await apiClient.update({
        project_id: projectId,
        agent_id: agentId,
        personality,
        expected_version: state.expectedVersion,
      });
      if (!mountedRef.current) return;
      onSaved?.(output.agent);
      onBack();
    } catch (error) {
      if (!mountedRef.current) return;
      setState((previous) => ({
        ...previous,
        isSubmitting: false,
        error: mapServiceError(error),
      }));
    }
  };

  const handleCancel = () => {
    if (!hasChanges()) {
      onBack();
      return;
    }
    if (window.confirm('Existem alterações não salvas. Deseja realmente sair?')) {
      onBack();
    }
  };

  const updateState = (patch: Partial<PersonalityFormState>) => {
    setState((previous) => ({ ...previous, ...patch, error: null }));
  };

  if (!agent) {
    return (
      <section className="personality-page" aria-label="Personalidade do Agent">
        <header className="personality-page__header">
          <button type="button" className="personality-page__back" onClick={onBack} aria-label="Voltar">
            ← Voltar
          </button>
          <h1>Personalidade do Agent</h1>
        </header>
        {state.error ? (
          <div className="personality-page__error" role="alert">
            <p>{state.error}</p>
            <button type="button" onClick={() => void fetchAgent()}>Tentar novamente</button>
          </div>
        ) : (
          <div className="personality-page__loading" role="status" aria-live="polite">
            Carregando personalidade...
          </div>
        )}
      </section>
    );
  }

  const isInactive = agent.status === 'inactive' || agent.status === 'suspended';
  const controlsDisabled = state.isSubmitting || isInactive;
  const preview = currentPersonality();

  return (
    <section className="personality-page" aria-label="Personalidade do Agent">
      <header className="personality-page__header">
        <button type="button" className="personality-page__back" onClick={handleCancel} aria-label="Voltar">
          ← Voltar
        </button>
        <h1>Personalidade do Agent</h1>
      </header>

      {state.error && (
        <div className="personality-page__error" role="alert">
          <p>{state.error}</p>
        </div>
      )}

      <div className="personality-page__boundary" role="note">
        <strong>Camada: Agent</strong>
        <span>Personalidade orienta estilo e tom; não substitui instruções de segurança ou sistema.</span>
      </div>

      <form
        className="personality-page__form"
        aria-label="Formulário de personalidade"
        noValidate
        onSubmit={handleSubmit}
      >
        <div className="personality-field">
          <label htmlFor="personality-name">Nome da personalidade</label>
          <input
            id="personality-name"
            type="text"
            value={state.name}
            onChange={(event) => updateState({ name: event.target.value })}
            maxLength={MAX_NAME_LENGTH}
            disabled={controlsDisabled}
            aria-describedby="personality-name-hint personality-name-count"
            required
          />
          <div className="personality-field__hint" id="personality-name-hint">
            Nome descritivo do estilo do agent.
            <span id="personality-name-count" aria-live="polite">
              {state.name.trim().length}/{MAX_NAME_LENGTH}
            </span>
          </div>
        </div>

        <div className="personality-field">
          <label htmlFor="personality-description">Descrição da personalidade</label>
          <textarea
            id="personality-description"
            value={state.description}
            onChange={(event) => updateState({ description: event.target.value })}
            maxLength={MAX_DESCRIPTION_LENGTH}
            rows={5}
            disabled={controlsDisabled}
            aria-describedby="personality-description-hint personality-description-count"
          />
          <div className="personality-field__hint" id="personality-description-hint">
            Descreva o comportamento e o tom, sem instruções de sistema ou credenciais.
            <span id="personality-description-count" aria-live="polite">
              {state.description.length}/{MAX_DESCRIPTION_LENGTH}
            </span>
          </div>
        </div>

        <div className="personality-field">
          <label htmlFor="personality-traits">Traços</label>
          <input
            id="personality-traits"
            type="text"
            value={state.traits.join(', ')}
            onChange={(event) => updateState({ traits: parseTraits(event.target.value) })}
            disabled={controlsDisabled}
            aria-describedby="personality-traits-hint personality-traits-count"
          />
          <div className="personality-field__hint" id="personality-traits-hint">
            Separe os traços por vírgulas; cada traço deve ser bounded e independente.
            <span id="personality-traits-count" aria-live="polite">
              {state.traits.length}/{MAX_TRAITS}
            </span>
          </div>
        </div>

        <div className="personality-field">
          <label htmlFor="communication-style">Estilo de comunicação</label>
          <select
            id="communication-style"
            value={state.communicationStyle}
            onChange={(event) => updateState({
              communicationStyle: event.target.value as Personality['communication_style'],
            })}
            disabled={controlsDisabled}
          >
            <option value="formal">Formal</option>
            <option value="casual">Casual</option>
            <option value="technical">Técnico</option>
            <option value="concise">Conciso</option>
            <option value="verbose">Detalhado</option>
          </select>
        </div>

        {isInactive && (
          <div className="personality-page__inactive" role="status">
            Este agent está arquivado e não pode ser editado; o status atual é inativo ou suspenso.
          </div>
        )}

        <section className="personality-page__preview" aria-label="Prévia segura">
          <h2>Prévia segura</h2>
          <p className="personality-page__preview-layer">Camada efetiva: Agent</p>
          <div data-testid="personality-preview" className="personality-page__preview-content">
            <strong>{preview.name || 'Sem nome'}</strong>
            <p>{preview.description || 'Sem descrição'}</p>
            <p>Traços: {preview.traits.length > 0 ? preview.traits.join(', ') : 'nenhum'}</p>
            <p>Estilo: {preview.communication_style}</p>
          </div>
        </section>

        <div className="personality-page__actions">
          <button type="button" onClick={handleCancel} disabled={state.isSubmitting}>
            Cancelar
          </button>
          <button
            type="submit"
            disabled={controlsDisabled || !hasChanges()}
          >
            {state.isSubmitting ? 'Salvando personalidade...' : 'Salvar personalidade'}
          </button>
        </div>
      </form>
    </section>
  );
};