import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  CapabilityState,
  defaultModelPolicyApi,
  ModelModality,
  ModelPolicy,
  ModelPolicySnapshot,
} from '@/api/agent-model-policy';
import { ModelPolicyFormState, ModelPolicyPageProps } from './types';
import './ModelPolicyPage.css';

const MODALITIES: ModelModality[] = ['text', 'image', 'audio', 'video'];
const MAX_PROVIDER_LENGTH = 120;
const MAX_MODEL_LENGTH = 200;

const EMPTY_FORM = {
  provider: '',
  model: '',
  maxTokens: '',
  maxContextTokens: '',
  temperature: '',
  modalities: [] as ModelModality[],
};

function formFromSnapshot(snapshot: ModelPolicySnapshot) {
  return {
    provider: snapshot.policy.provider,
    model: snapshot.policy.model,
    maxTokens: snapshot.policy.max_tokens?.toString() ?? '',
    maxContextTokens: snapshot.policy.max_context_tokens?.toString() ?? '',
    temperature: snapshot.policy.temperature?.toString() ?? '',
    modalities: [...snapshot.policy.modalities],
  };
}

function policyFromState(state: ModelPolicyFormState): ModelPolicy {
  const policy: ModelPolicy = {
    provider: state.provider.trim(),
    model: state.model.trim(),
    modalities: [...state.modalities],
  };

  if (state.maxTokens.trim() !== '') policy.max_tokens = Number(state.maxTokens);
  if (state.maxContextTokens.trim() !== '') {
    policy.max_context_tokens = Number(state.maxContextTokens);
  }
  if (state.temperature.trim() !== '') policy.temperature = Number(state.temperature);
  return policy;
}

function sameForm(left: ModelPolicyFormState['initialData'], right: ModelPolicyFormState['initialData']) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function mapError(error: unknown): string {
  const message = error instanceof Error ? error.message : 'Falha ao salvar política de modelo';
  const normalized = message.toLocaleLowerCase('en-US');
  if (normalized.includes('stale') || normalized.includes('version') || normalized.includes('concurrency')) {
    return 'A política de modelo foi modificada por outro processo. Recarregue e tente novamente.';
  }
  if (normalized.includes('unsupported') || normalized.includes('provider')) {
    return 'Nenhum provider compatível está disponível para esta política.';
  }
  return message;
}

function validateInteger(value: string, label: string, min: number, max: number): string | null {
  if (value.trim() === '') return null;
  if (!/^\d+$/.test(value.trim())) return `${label} deve ser um número inteiro`;
  const numeric = Number(value);
  if (!Number.isSafeInteger(numeric) || numeric < min || numeric > max) {
    return `${label} deve estar entre ${min} e ${max}`;
  }
  return null;
}

export const ModelPolicyPage: React.FC<ModelPolicyPageProps> = ({
  projectId,
  agentId,
  onBack,
  onSaved,
  apiClient = defaultModelPolicyApi,
}) => {
  const [snapshot, setSnapshot] = useState<ModelPolicySnapshot | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [state, setState] = useState<ModelPolicyFormState>({
    ...EMPTY_FORM,
    expectedVersion: '',
    initialData: EMPTY_FORM,
    isSubmitting: false,
    error: null,
  });
  const mountedRef = useRef(true);

  const fetchPolicy = useCallback(async () => {
    setLoaded(false);
    setState((previous) => ({ ...previous, error: null }));
    try {
      const fetched = await apiClient.get(projectId, agentId);
      if (!mountedRef.current) return;
      setSnapshot(fetched);
      setLoaded(true);
      if (fetched) {
        const form = formFromSnapshot(fetched);
        setState((previous) => ({
          ...previous,
          ...form,
          initialData: form,
          expectedVersion: fetched.updated_at,
          error: null,
        }));
      }
    } catch {
      if (!mountedRef.current) return;
      setLoaded(true);
      setState((previous) => ({ ...previous, error: 'Falha ao carregar política de modelo' }));
    }
  }, [agentId, apiClient, projectId]);

  useEffect(() => {
    mountedRef.current = true;
    void fetchPolicy();
    return () => {
      mountedRef.current = false;
    };
  }, [fetchPolicy]);

  const currentForm = () => ({
    provider: state.provider.trim(),
    model: state.model.trim(),
    maxTokens: state.maxTokens.trim(),
    maxContextTokens: state.maxContextTokens.trim(),
    temperature: state.temperature.trim(),
    modalities: [...state.modalities],
  });

  const hasChanges = () => !sameForm(currentForm(), state.initialData);

  const validate = (): string | null => {
    const provider = state.provider.trim();
    const model = state.model.trim();
    if (!provider || !model) return 'Provider ID e Model ID abstratos são obrigatórios';
    if (provider.length > MAX_PROVIDER_LENGTH) {
      return `Provider ID abstrato deve ter no máximo ${MAX_PROVIDER_LENGTH} caracteres`;
    }
    if (model.length > MAX_MODEL_LENGTH) {
      return `Model ID abstrato deve ter no máximo ${MAX_MODEL_LENGTH} caracteres`;
    }
    if (provider.includes('://') || model.includes('://')) {
      return 'Identificador inválido: URL e endpoint não são permitidos';
    }

    const maxTokensError = validateInteger(state.maxTokens, 'Max tokens', 1, 1_000_000);
    if (maxTokensError) return maxTokensError;
    const maxContextError = validateInteger(
      state.maxContextTokens,
      'Janela de contexto',
      1,
      2_000_000,
    );
    if (maxContextError) return maxContextError;

    if (state.temperature.trim() !== '') {
      const temperature = Number(state.temperature);
      if (!Number.isFinite(temperature) || temperature < 0 || temperature > 2) {
        return 'Temperatura deve estar entre 0 e 2';
      }
    }
    if (state.modalities.length === 0) return 'Selecione pelo menos uma modalidade';
    if (new Set(state.modalities).size !== state.modalities.length) {
      return 'Modalidades duplicadas não são permitidas';
    }
    return null;
  };

  const updateState = (patch: Partial<ModelPolicyFormState>) => {
    setState((previous) => ({ ...previous, ...patch, error: null }));
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const validationError = validate();
    if (validationError) {
      setState((previous) => ({ ...previous, error: validationError }));
      return;
    }
    if (!hasChanges()) return;

    setState((previous) => ({ ...previous, isSubmitting: true, error: null }));
    try {
      const updated = await apiClient.update({
        project_id: projectId,
        agent_id: agentId,
        policy: policyFromState(state),
        expected_version: state.expectedVersion,
      });
      if (!mountedRef.current) return;
      setSnapshot(updated);
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

  const toggleModality = (modality: ModelModality) => {
    const modalities = state.modalities.includes(modality)
      ? state.modalities.filter((item) => item !== modality)
      : [...state.modalities, modality];
    updateState({ modalities });
  };

  if (!loaded) {
    return (
      <section className="model-policy-page" aria-label="Política de modelo do Agent">
        <header className="model-policy-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Política de modelo do Agent</h1>
        </header>
        <div role="status" aria-live="polite" className="model-policy-page__loading">
          Carregando política de modelo...
        </div>
      </section>
    );
  }

  if (!snapshot) {
    return (
      <section className="model-policy-page" aria-label="Política de modelo do Agent">
        <header className="model-policy-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Política de modelo do Agent</h1>
        </header>
        {state.error ? (
          <div role="alert" className="model-policy-page__error">
            <p>{state.error}</p>
            <button type="button" onClick={() => void fetchPolicy()}>Tentar novamente</button>
          </div>
        ) : (
          <div role="status" className="model-policy-page__unsupported">
            <strong>Nenhum provider disponível para este agent.</strong>
            <span>O suporte de modelo é desconhecido ou indisponível; nenhuma capacidade foi inventada.</span>
          </div>
        )}
      </section>
    );
  }

  const capabilityEntries = MODALITIES.map((modality) => [modality, snapshot.capabilities[modality]] as const);
  const controlsDisabled = state.isSubmitting;

  return (
    <section className="model-policy-page" aria-label="Política de modelo do Agent">
      <header className="model-policy-page__header">
        <button type="button" onClick={handleCancel} aria-label="Voltar" disabled={state.isSubmitting}>
          ← Voltar
        </button>
        <h1>Política de modelo do Agent</h1>
      </header>

      {state.error && <div role="alert" className="model-policy-page__error"><p>{state.error}</p></div>}

      <div className="model-policy-page__boundary" role="note">
        <strong>Provider-neutral</strong>
        <span>Policy provider-neutral, sem SDK concreto, endpoint, rede ou credencial.</span>
      </div>

      {snapshot.provider_state === 'unsupported' && (
        <div className="model-policy-page__unsupported" role="status">
          Nenhum provider compatível foi negociado; a policy permanece explícita e provider-neutral.
        </div>
      )}
      {snapshot.provider_state === 'unknown' && (
        <div className="model-policy-page__unsupported" role="status">
          O estado do provider é desconhecido; não será tratado como suporte confirmado.
        </div>
      )}

      <form
        className="model-policy-page__form"
        aria-label="Formulário de política de modelo"
        noValidate
        onSubmit={handleSubmit}
      >
        <div className="model-policy-field">
          <label htmlFor="model-provider">Provider ID abstrato</label>
          <input
            id="model-provider"
            type="text"
            value={state.provider}
            onChange={(event) => updateState({ provider: event.target.value })}
            maxLength={MAX_PROVIDER_LENGTH}
            disabled={controlsDisabled}
            required
          />
        </div>

        <div className="model-policy-field">
          <label htmlFor="model-id">Model ID abstrato</label>
          <input
            id="model-id"
            type="text"
            value={state.model}
            onChange={(event) => updateState({ model: event.target.value })}
            maxLength={MAX_MODEL_LENGTH}
            disabled={controlsDisabled}
            required
          />
        </div>

        <div className="model-policy-grid">
          <div className="model-policy-field">
            <label htmlFor="model-max-tokens">Max tokens</label>
            <input
              id="model-max-tokens"
              type="number"
              min="1"
              max="1000000"
              step="1"
              value={state.maxTokens}
              onChange={(event) => updateState({ maxTokens: event.target.value })}
            />
          </div>
          <div className="model-policy-field">
            <label htmlFor="model-context">Janela de contexto (tokens)</label>
            <input
              id="model-context"
              type="number"
              min="1"
              max="2000000"
              step="1"
              value={state.maxContextTokens}
              onChange={(event) => updateState({ maxContextTokens: event.target.value })}
            />
          </div>
          <div className="model-policy-field">
            <label htmlFor="model-temperature">Temperatura</label>
            <input
              id="model-temperature"
              type="number"
              min="0"
              max="2"
              step="0.1"
              value={state.temperature}
              onChange={(event) => updateState({ temperature: event.target.value })}
            />
          </div>
        </div>

        <fieldset className="model-policy-modalities" disabled={controlsDisabled}>
          <legend>Modalidades requeridas</legend>
          <div className="model-policy-modalities__options">
            {MODALITIES.map((modality) => (
              <label key={modality}>
                <input
                  type="checkbox"
                  checked={state.modalities.includes(modality)}
                  onChange={() => toggleModality(modality)}
                  aria-label={`Modalidade ${modality}`}
                />
                {modality}
              </label>
            ))}
          </div>
          <small>Suporte desconhecido não é convertido em suporte confirmado.</small>
        </fieldset>

        <section className="model-policy-capabilities" aria-label="Dicas de capability">
          <h2>Estado de capability</h2>
          <ul>
            {capabilityEntries.map(([modality, capability]) => (
              <li key={modality} className={`capability capability--${capability}`}>
                {modality}: {capability as CapabilityState}
              </li>
            ))}
          </ul>
        </section>

        <div className="model-policy-page__actions">
          <button type="button" onClick={handleCancel} disabled={state.isSubmitting}>Cancelar</button>
          <button type="submit" disabled={controlsDisabled || !hasChanges()}>
            {state.isSubmitting ? 'Salvando política...' : 'Salvar política de modelo'}
          </button>
        </div>
      </form>
    </section>
  );
};