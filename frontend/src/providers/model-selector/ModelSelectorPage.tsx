import { useEffect, useMemo, useState } from 'react';
import {
  defaultModelSelectorApi,
  isModelOptionCompatible,
  ModelSelectorApiClient,
  ModelSelectorOption,
  ModelSelectorSnapshot,
  optionKey,
  UpdateModelSelectionInput,
} from '../../api/model-selector';
import './ModelSelectorPage.css';

export type {
  ModelSelectorApiClient,
  ModelSelectorOption,
  ModelSelectorSnapshot,
} from '../../api/model-selector';

interface ModelSelectorPageProps {
  projectId: string;
  agentId: string;
  apiClient?: ModelSelectorApiClient;
  onBack: () => void;
  onSaved?: (snapshot: ModelSelectorSnapshot) => void;
}

function safeReason(option: ModelSelectorOption): string | null {
  if (!option.reason) return null;
  if (/api[_-]?key|authorization|bearer|secret|token|https?:\/\//i.test(option.reason)) {
    return 'Detalhe indisponível por segurança.';
  }
  return option.reason;
}

function stateLabel(option: ModelSelectorOption): string {
  if (option.state === 'disabled') return 'Provider desabilitado';
  if (option.state === 'expired') return 'Credential expirada';
  if (option.state === 'unavailable') return 'Indisponível';
  return 'Disponível';
}

function errorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message.toLocaleLowerCase('en-US') : '';
  if (message.includes('stale') || message.includes('version') || message.includes('concurrency')) {
    return 'A seleção foi modificada por outro processo. Recarregue e tente novamente.';
  }
  if (message.includes('unsupported') || message.includes('capability')) {
    return 'O modelo deixou de atender às capabilities exigidas pela policy.';
  }
  return 'Não foi possível salvar a seleção de modelo.';
}

export function ModelSelectorPage({
  projectId,
  agentId,
  apiClient = defaultModelSelectorApi,
  onBack,
  onSaved,
}: ModelSelectorPageProps) {
  const [snapshot, setSnapshot] = useState<ModelSelectorSnapshot | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setLoaded(false);
    setError(null);
    apiClient
      .get(projectId, agentId)
      .then((fetched) => {
        if (!active) return;
        setSnapshot(fetched);
        const current = fetched.options.find(
          (option) => option.provider_id === fetched.policy.provider && option.model_id === fetched.policy.model,
        );
        setSelected(current && isModelOptionCompatible(current, fetched.policy.modalities) ? optionKey(current) : null);
      })
      .catch(() => {
        if (active) setError('Não foi possível carregar os modelos descobertos.');
      })
      .finally(() => {
        if (active) setLoaded(true);
      });
    return () => {
      active = false;
    };
  }, [agentId, apiClient, projectId]);

  const compatibleOptions = useMemo(() => {
    if (!snapshot) return [];
    return snapshot.options.filter((option) => isModelOptionCompatible(option, snapshot.policy.modalities));
  }, [snapshot]);

  const choose = (option: ModelSelectorOption) => {
    if (!snapshot || !isModelOptionCompatible(option, snapshot.policy.modalities)) return;
    setSelected(optionKey(option));
    setError(null);
  };

  const save = async () => {
    if (!snapshot || !selected) {
      setError('Selecione um modelo compatível antes de salvar.');
      return;
    }
    const option = snapshot.options.find((candidate) => optionKey(candidate) === selected);
    if (!option || !isModelOptionCompatible(option, snapshot.policy.modalities)) {
      setError('A opção selecionada não possui capabilities confirmadas.');
      return;
    }
    const input: UpdateModelSelectionInput = {
      project_id: projectId,
      agent_id: agentId,
      provider_id: option.provider_id,
      model_id: option.model_id,
      expected_version: snapshot.updated_at,
    };
    setSaving(true);
    setError(null);
    try {
      const updated = await apiClient.update(input);
      setSnapshot(updated);
      onSaved?.(updated);
      onBack();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setSaving(false);
    }
  };

  if (!loaded) {
    return (
      <main className="model-selector-page" aria-label="Seleção de modelo">
        <header className="model-selector-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Selecionar modelo</h1>
        </header>
        <p role="status" aria-live="polite">Carregando modelos compatíveis...</p>
      </main>
    );
  }

  if (!snapshot) {
    return (
      <main className="model-selector-page" aria-label="Seleção de modelo">
        <header className="model-selector-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Selecionar modelo</h1>
        </header>
        <div role="alert" className="model-selector-page__alert error">
          {error ?? 'Nenhum resultado de discovery disponível.'}
        </div>
      </main>
    );
  }

  return (
    <main className="model-selector-page" aria-label="Seleção de modelo">
      <header className="model-selector-page__header">
        <div>
          <p className="model-selector-page__eyebrow">Agent {agentId}</p>
          <h1>Selecionar modelo</h1>
          <p>A seleção usa somente modelos descobertos e capabilities confirmadas para este projeto.</p>
        </div>
        <button type="button" onClick={onBack} disabled={saving}>Voltar</button>
      </header>

      {error && <div role="alert" className="model-selector-page__alert error">{error}</div>}

      {compatibleOptions.length === 0 && (
        <div role="status" className="model-selector-page__empty">
          <strong>Nenhum modelo compatível disponível.</strong>
          <span>Capabilities desconhecidas, providers indisponíveis ou credenciais expiradas não são aceitos; não haverá fallback automático.</span>
        </div>
      )}

      <fieldset className="model-selector-page__options" role="radiogroup" aria-label="Modelos disponíveis">
        <legend>Modelos descobertos</legend>
        {snapshot.options.map((option) => {
          const compatible = isModelOptionCompatible(option, snapshot.policy.modalities);
          const key = optionKey(option);
          const reason = safeReason(option);
          const distinctReason = reason === stateLabel(option) ? null : reason;
          return (
            <label className={`model-selector-option ${compatible ? 'is-compatible' : 'is-unavailable'}`} key={key}>
              <input
                type="radio"
                name="model-selection"
                value={key}
                checked={selected === key}
                disabled={!compatible || saving}
                onChange={() => choose(option)}
              />
              <span className="model-selector-option__body">
                <strong>{option.display_name}</strong>
                <span>{option.provider_id} · {option.model_id}</span>
                <span className="model-selector-option__state">{stateLabel(option)} · {option.source === 'cache' ? 'cache' : 'provider'}</span>
                {!compatible && <span className="model-selector-option__reason">{distinctReason ?? (reason ? null : 'Não atende às capabilities exigidas pela policy.')}</span>}
              </span>
            </label>
          );
        })}
      </fieldset>

      <div className="model-selector-page__actions">
        <button type="button" onClick={() => void save()} disabled={saving || !selected}>
          {saving ? 'Salvando...' : 'Salvar seleção de modelo'}
        </button>
      </div>
    </main>
  );
}
