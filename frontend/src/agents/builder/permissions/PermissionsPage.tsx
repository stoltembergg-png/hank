import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  defaultToolPermissionApi,
  PermissionAction,
  PermissionEffect,
  PermissionResource,
  PermissionScope,
  PermissionRule,
  ToolPermissionPolicy,
} from '@/api/agent-tool-permissions';
import { PermissionDraft, PermissionsFormState, PermissionsPageProps } from './types';
import './PermissionsPage.css';

const SCHEMA_VERSION = 1;
const MAX_RULES = 128;
const MAX_SCOPE_LENGTH = 160;

const RESOURCES: PermissionResource[] = [
  'project',
  'agent',
  'session',
  'message',
  'memory',
  'skill',
  'tool',
  'workflow',
  'file',
  'process',
  'network',
  'secret',
  'provider',
  'plugin',
  'remote_node',
  'settings',
];
const ACTIONS: PermissionAction[] = [
  'create',
  'read',
  'update',
  'delete',
  'list',
  'execute',
  'invoke',
  'delegate',
  'approve',
  'revoke',
  'configure',
  'discover',
  'stream',
  'cancel',
  'retry',
];
const SCOPES: PermissionScope[] = ['project', 'agent', 'session'];
const EFFECTS: PermissionEffect[] = ['allow', 'ask', 'deny'];

const DEFAULT_DRAFT: PermissionDraft = {
  resource: 'tool',
  action: 'invoke',
  effect: 'ask',
  scope: 'project',
  scopeId: '',
  expiresAt: '',
};

function ruleKey(rule: PermissionRule): string {
  return `${rule.capability.resource}:${rule.capability.action}:${rule.scope}:${rule.scope_id}`;
}

function isApprovalSensitive(rule: Pick<PermissionRule, 'capability' | 'effect'>): boolean {
  const resource = rule.capability.resource;
  const action = rule.capability.action;
  return (
    resource === 'secret'
    || (['file', 'process', 'network'].includes(resource)
      && ['execute', 'invoke', 'delete'].includes(action))
  );
}

function isPrivilegedWildcard(rule: PermissionRule): boolean {
  return (
    ['file', 'process', 'network'].includes(rule.capability.resource)
    && ['execute', 'invoke', 'delete'].includes(rule.capability.action)
    && (!rule.capability.scope || rule.capability.scope === '*')
  );
}

function validateRule(rule: PermissionRule): string | null {
  if (!rule.scope_id.trim() || rule.scope_id.length > MAX_SCOPE_LENGTH || /[\n*]/.test(rule.scope_id)) {
    return 'Escopo inválido: ID vazio, longo, com newline ou wildcard';
  }
  if (!rule.capability.scope || /[\n*]/.test(rule.capability.scope)) {
    return 'Capability ampla com wildcard não é permitida';
  }
  if (isPrivilegedWildcard(rule)) {
    return 'Capability privilegiada com wildcard não é permitida';
  }
  if (isApprovalSensitive(rule) && rule.effect === 'allow') {
    return 'Grants destrutivos ou sensíveis exigem aprovação explícita (efeito ask) ou deny';
  }
  return null;
}

function validatePolicy(policy: ToolPermissionPolicy): string | null {
  if (policy.schema_version !== SCHEMA_VERSION) return 'Versão de schema de permissões inválida';
  if (policy.default_effect !== 'deny') return 'Política inválida: default deny é obrigatório';
  if (policy.rules.length > MAX_RULES) return `A política aceita no máximo ${MAX_RULES} regras`;

  const keys = new Set<string>();
  for (const rule of policy.rules) {
    const error = validateRule(rule);
    if (error) return error;
    const key = ruleKey(rule);
    if (keys.has(key)) return 'Política inválida: regra conflitante duplicada';
    keys.add(key);
  }
  return null;
}

function mapError(error: unknown): string {
  const message = error instanceof Error ? error.message : 'Falha ao salvar permissões';
  const normalized = message.toLocaleLowerCase('en-US');
  if (normalized.includes('stale') || normalized.includes('version') || normalized.includes('concurrency')) {
    return 'A política de permissões foi modificada por outro processo. Recarregue e tente novamente.';
  }
  if (normalized.includes('permission') || normalized.includes('forbidden')) {
    return 'Você não tem permissão para editar esta política de permissões.';
  }
  return message;
}

export const PermissionsPage: React.FC<PermissionsPageProps> = ({
  projectId,
  agentId,
  onBack,
  onSaved,
  apiClient = defaultToolPermissionApi,
}) => {
  const [loaded, setLoaded] = useState(false);
  const [state, setState] = useState<PermissionsFormState>({
    policy: null,
    initialPolicy: null,
    expectedVersion: '',
    draft: DEFAULT_DRAFT,
    isSubmitting: false,
    error: null,
  });
  const mountedRef = useRef(true);

  const fetchPermissions = useCallback(async () => {
    setLoaded(false);
    setState((previous) => ({ ...previous, error: null }));
    try {
      const fetched = await apiClient.get(projectId, agentId);
      if (!mountedRef.current) return;
      setLoaded(true);

      if (!fetched) {
        setState((previous) => ({
          ...previous,
          policy: null,
          initialPolicy: null,
          expectedVersion: '',
        }));
        return;
      }

      const policyError = validatePolicy(fetched.policy);
      if (policyError) {
        setState((previous) => ({
          ...previous,
          policy: null,
          initialPolicy: null,
          expectedVersion: '',
          error: policyError,
        }));
        return;
      }

      setState((previous) => ({
        ...previous,
        policy: fetched.policy,
        initialPolicy: fetched.policy,
        expectedVersion: fetched.updated_at,
        error: null,
      }));
    } catch {
      if (!mountedRef.current) return;
      setLoaded(true);
      setState((previous) => ({ ...previous, error: 'Falha ao carregar permissões' }));
    }
  }, [agentId, apiClient, projectId]);

  useEffect(() => {
    mountedRef.current = true;
    void fetchPermissions();
    return () => {
      mountedRef.current = false;
    };
  }, [fetchPermissions]);

  const hasChanges = () => JSON.stringify(state.policy) !== JSON.stringify(state.initialPolicy);

  const updateDraft = (patch: Partial<PermissionDraft>) => {
    setState((previous) => ({
      ...previous,
      draft: { ...previous.draft, ...patch },
      error: null,
    }));
  };

  const draftRule = (): PermissionRule => ({
    capability: {
      resource: state.draft.resource,
      action: state.draft.action,
      scope: state.draft.scopeId.trim(),
    },
    effect: state.draft.effect,
    scope: state.draft.scope,
    scope_id: state.draft.scopeId.trim(),
    expires_at: state.draft.expiresAt ? new Date(state.draft.expiresAt).toISOString() : null,
  });

  const handleAddRule = () => {
    if (!state.policy) return;
    const rule = draftRule();
    const validationError = validateRule(rule);
    if (validationError) {
      setState((previous) => ({ ...previous, error: validationError }));
      return;
    }
    if (state.policy.rules.some((existing) => ruleKey(existing) === ruleKey(rule))) {
      setState((previous) => ({
        ...previous,
        error: 'Regra conflitante duplicada não pode ser adicionada',
      }));
      return;
    }
    if (state.policy.rules.length >= MAX_RULES) {
      setState((previous) => ({ ...previous, error: `A política aceita no máximo ${MAX_RULES} regras` }));
      return;
    }

    setState((previous) => ({
      ...previous,
      policy: previous.policy
        ? { ...previous.policy, rules: [...previous.policy.rules, rule] }
        : previous.policy,
      error: null,
    }));
  };

  const handleRemoveRule = (rule: PermissionRule) => {
    setState((previous) => ({
      ...previous,
      policy: previous.policy
        ? { ...previous.policy, rules: previous.policy.rules.filter((item) => ruleKey(item) !== ruleKey(rule)) }
        : previous.policy,
      error: null,
    }));
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!state.policy) return;
    const policyError = validatePolicy(state.policy);
    if (policyError) {
      setState((previous) => ({ ...previous, error: policyError }));
      return;
    }
    if (!hasChanges()) return;

    setState((previous) => ({ ...previous, isSubmitting: true, error: null }));
    try {
      const updated = await apiClient.update({
        project_id: projectId,
        agent_id: agentId,
        policy: state.policy,
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
      <section className="permissions-page" aria-label="Políticas de permissões do Agent">
        <header className="permissions-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Políticas de permissões do Agent</h1>
        </header>
        <div className="permissions-page__loading" role="status" aria-live="polite">
          Carregando permissões...
        </div>
      </section>
    );
  }

  if (!state.policy) {
    return (
      <section className="permissions-page" aria-label="Políticas de permissões do Agent">
        <header className="permissions-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Políticas de permissões do Agent</h1>
        </header>
        {state.error ? (
          <div className="permissions-page__error" role="alert">
            <p>{state.error}</p>
            <button type="button" onClick={() => void fetchPermissions()}>Tentar novamente</button>
          </div>
        ) : (
          <div className="permissions-page__unsupported" role="status">
            <strong>Nenhum Permission Engine disponível para este agent.</strong>
            <span>O default deny permanece implícito; nenhuma permissão é concedida automaticamente.</span>
          </div>
        )}
      </section>
    );
  }

  return (
    <section className="permissions-page" aria-label="Políticas de permissões do Agent">
      <header className="permissions-page__header">
        <button type="button" onClick={handleCancel} aria-label="Voltar" disabled={state.isSubmitting}>
          ← Voltar
        </button>
        <h1>Políticas de permissões do Agent</h1>
      </header>

      {state.error && <div className="permissions-page__error" role="alert"><p>{state.error}</p></div>}

      <div className="permissions-page__boundary" role="note">
        <strong>Default effect: deny</strong>
        <span>Least privilege: sem wildcard padrão, sem executor, sem grant automático e com aprovação explícita para operações sensíveis.</span>
      </div>

      <form className="permissions-page__form" aria-label="Formulário de permissões" noValidate onSubmit={handleSubmit}>
        <fieldset className="permissions-page__draft" disabled={state.isSubmitting}>
          <legend>Adicionar regra</legend>
          <div className="permissions-page__grid">
            <label>
              Recurso
              <select value={state.draft.resource} onChange={(event) => updateDraft({ resource: event.target.value as PermissionResource })}>
                {RESOURCES.map((resource) => <option key={resource} value={resource}>{resource}</option>)}
              </select>
            </label>
            <label>
              Ação
              <select value={state.draft.action} onChange={(event) => updateDraft({ action: event.target.value as PermissionAction })}>
                {ACTIONS.map((action) => <option key={action} value={action}>{action}</option>)}
              </select>
            </label>
            <label>
              Efeito
              <select value={state.draft.effect} onChange={(event) => updateDraft({ effect: event.target.value as PermissionEffect })}>
                {EFFECTS.map((effect) => <option key={effect} value={effect}>{effect}</option>)}
              </select>
            </label>
            <label>
              Escopo
              <select value={state.draft.scope} onChange={(event) => updateDraft({ scope: event.target.value as PermissionScope })}>
                {SCOPES.map((scope) => <option key={scope} value={scope}>{scope}</option>)}
              </select>
            </label>
          </div>
          <label>
            ID do escopo
            <input type="text" value={state.draft.scopeId} onChange={(event) => updateDraft({ scopeId: event.target.value })} placeholder="project/agent/session id" />
          </label>
          <label>
            Expiração (opcional)
            <input type="datetime-local" value={state.draft.expiresAt} onChange={(event) => updateDraft({ expiresAt: event.target.value })} />
          </label>
          <p className="permissions-page__hint">Efeito <strong>ask</strong> significa aprovação humana explícita antes da execução.</p>
          <button type="button" onClick={handleAddRule}>Adicionar regra</button>
        </fieldset>

        <section className="permissions-page__effective" aria-label="Regras efetivas">
          <h2>Regras efetivas</h2>
          <p>Default: deny</p>
          {state.policy.rules.length === 0 ? (
            <p>Nenhuma regra explícita; todas as capabilities permanecem negadas.</p>
          ) : (
            <ul>
              {state.policy.rules.map((rule) => (
                <li key={ruleKey(rule)}>
                  <div>
                    <strong>{rule.capability.resource}:{rule.capability.action}</strong>
                    <span>{rule.scope}:{rule.scope_id}</span>
                    <span>Regra efetiva: {rule.effect}</span>
                    {rule.effect === 'ask' && <span>Aprovação humana necessária</span>}
                    {rule.expires_at && <span>Expira: {new Date(rule.expires_at).toLocaleString()}</span>}
                  </div>
                  <button type="button" onClick={() => handleRemoveRule(rule)} disabled={state.isSubmitting} aria-label={`Remover regra ${rule.capability.resource}:${rule.capability.action}`}>
                    Remover
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <div className="permissions-page__actions">
          <button type="button" onClick={handleCancel} disabled={state.isSubmitting}>Cancelar</button>
          <button type="submit" disabled={state.isSubmitting || !hasChanges()}>
            {state.isSubmitting ? 'Salvando permissões...' : 'Salvar permissões'}
          </button>
        </div>
      </form>
    </section>
  );
};