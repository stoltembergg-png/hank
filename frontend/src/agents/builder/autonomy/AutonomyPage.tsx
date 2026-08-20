import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  AutonomyDecision,
  AutonomyLevel,
  AutonomyOperation,
  AutonomyPolicy,
  AutonomyPolicySnapshot,
  defaultAutonomyApi,
} from '@/api/agent-autonomy';
import { AutonomyFormState, AutonomyPageProps } from './types';
import './AutonomyPage.css';

const LEVELS: AutonomyLevel[] = [
  'l0_none',
  'l1_assisted',
  'l2_semi_autonomous',
  'l3_autonomous',
  'l4_fully_autonomous',
];
const OPERATIONS: AutonomyOperation[] = [
  'read_data',
  'execute_safe_tool',
  'execute_stateful_tool',
  'spawn_sub_agent',
  'create_workflow',
  'modify_skill',
  'access_external_network',
  'modify_system_config',
];
const LEVEL_LABELS: Record<AutonomyLevel, string> = {
  l0_none: 'L0 — Nenhum',
  l1_assisted: 'L1 — Assistido',
  l2_semi_autonomous: 'L2 — Semi-autônomo',
  l3_autonomous: 'L3 — Autônomo',
  l4_fully_autonomous: 'L4 — Totalmente autônomo',
};
const LEVEL_RANK: Record<AutonomyLevel, number> = {
  l0_none: 0,
  l1_assisted: 1,
  l2_semi_autonomous: 2,
  l3_autonomous: 3,
  l4_fully_autonomous: 4,
};
const DECISIONS: AutonomyDecision[] = ['allow', 'require_human_approval', 'deny'];

function defaultsForLevel(level: AutonomyLevel): AutonomyPolicy {
  const defaults: Record<AutonomyLevel, AutonomyPolicy> = {
    l0_none: {
      schema_version: 1,
      level,
      allow_subagents: false,
      allow_workflow_creation: false,
      allow_skill_modification: false,
      allow_network_access: false,
      max_consecutive_autonomous_steps: 1,
    },
    l1_assisted: {
      schema_version: 1,
      level,
      allow_subagents: false,
      allow_workflow_creation: false,
      allow_skill_modification: false,
      allow_network_access: false,
      max_consecutive_autonomous_steps: 10,
    },
    l2_semi_autonomous: {
      schema_version: 1,
      level,
      allow_subagents: true,
      allow_workflow_creation: true,
      allow_skill_modification: false,
      allow_network_access: false,
      max_consecutive_autonomous_steps: 50,
    },
    l3_autonomous: {
      schema_version: 1,
      level,
      allow_subagents: true,
      allow_workflow_creation: true,
      allow_skill_modification: true,
      allow_network_access: true,
      max_consecutive_autonomous_steps: 200,
    },
    l4_fully_autonomous: {
      schema_version: 1,
      level,
      allow_subagents: true,
      allow_workflow_creation: true,
      allow_skill_modification: true,
      allow_network_access: true,
      max_consecutive_autonomous_steps: 1000,
    },
  };
  return defaults[level];
}

function validateSnapshot(snapshot: AutonomyPolicySnapshot): string | null {
  const expected = defaultsForLevel(snapshot.policy.level);
  if (snapshot.policy.schema_version !== 1) return 'Versão de schema de autonomia inválida';
  if (snapshot.policy.max_consecutive_autonomous_steps < 1 || snapshot.policy.max_consecutive_autonomous_steps > 1000) {
    return 'Limite de passos autônomos inválido';
  }
  if (snapshot.policy.level === 'l0_none' && (
    snapshot.policy.allow_subagents
    || snapshot.policy.allow_workflow_creation
    || snapshot.policy.allow_skill_modification
    || snapshot.policy.allow_network_access
    || snapshot.policy.max_consecutive_autonomous_steps > 1
  )) {
    return 'Política inválida: L0 não pode habilitar execução autônoma';
  }
  if (snapshot.policy.level === 'l1_assisted' && (
    snapshot.policy.allow_subagents
    || snapshot.policy.allow_workflow_creation
    || snapshot.policy.allow_skill_modification
    || snapshot.policy.allow_network_access
  )) {
    return 'Política inválida: L1 não pode habilitar flags avançadas';
  }
  if (snapshot.policy.level === 'l2_semi_autonomous' && (
    snapshot.policy.allow_skill_modification || snapshot.policy.allow_network_access
  )) {
    return 'Política inválida: L2 não pode habilitar skill/network sem escalação';
  }
  if (snapshot.policy.level === 'l0_none' && snapshot.policy.max_consecutive_autonomous_steps !== expected.max_consecutive_autonomous_steps) {
    return 'Política inválida: limite de L0 deve ser 1';
  }
  if (OPERATIONS.some((operation) => !DECISIONS.includes(snapshot.decisions[operation]))) {
    return 'Matriz de decisões de autonomia inválida';
  }
  return null;
}

function mapError(error: unknown): string {
  const message = error instanceof Error ? error.message : 'Falha ao salvar política de autonomia';
  const normalized = message.toLocaleLowerCase('en-US');
  if (normalized.includes('stale') || normalized.includes('version') || normalized.includes('concurrency')) {
    return 'A política de autonomia foi modificada por outro processo. Recarregue e tente novamente.';
  }
  if (normalized.includes('permission') || normalized.includes('approval')) {
    return 'A escalação de autonomia foi negada: aprovação humana explícita é necessária.';
  }
  return message;
}

export const AutonomyPage: React.FC<AutonomyPageProps> = ({
  projectId,
  agentId,
  onBack,
  onSaved,
  apiClient = defaultAutonomyApi,
}) => {
  const [loaded, setLoaded] = useState(false);
  const [snapshot, setSnapshot] = useState<AutonomyPolicySnapshot | null>(null);
  const [state, setState] = useState<AutonomyFormState>({
    targetLevel: 'l1_assisted',
    maxSteps: '10',
    approverId: '',
    reason: '',
    expiresAt: '',
    initialLevel: 'l1_assisted',
    initialMaxSteps: 10,
    expectedVersion: '',
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
        targetLevel: fetched.policy.level,
        maxSteps: String(fetched.policy.max_consecutive_autonomous_steps),
        initialLevel: fetched.policy.level,
        initialMaxSteps: fetched.policy.max_consecutive_autonomous_steps,
        expectedVersion: fetched.updated_at,
        error: null,
      }));
    } catch {
      if (!mountedRef.current) return;
      setLoaded(true);
      setState((previous) => ({ ...previous, error: 'Falha ao carregar política de autonomia' }));
    }
  }, [agentId, apiClient, projectId]);

  useEffect(() => {
    mountedRef.current = true;
    void fetchPolicy();
    return () => {
      mountedRef.current = false;
    };
  }, [fetchPolicy]);

  const isEscalation = LEVEL_RANK[state.targetLevel] > LEVEL_RANK[state.initialLevel];
  const hasChanges = () => (
    state.targetLevel !== state.initialLevel
    || Number(state.maxSteps) !== state.initialMaxSteps
  );

  const validate = (): string | null => {
    const steps = Number(state.maxSteps);
    if (!Number.isInteger(steps) || steps < 1 || steps > 1000) {
      return 'Limite de passos autônomos deve estar entre 1 e 1000';
    }
    if (!hasChanges()) return null;
    if (!isEscalation) return null;
    if (!state.approverId.trim() || state.approverId.trim().length > 128) {
      return 'Escalação exige aprovação humana explícita: informe Approver ID válido';
    }
    if (!state.reason.trim() || state.reason.trim().length > 256) {
      return 'Escalação exige motivo de aprovação válido';
    }
    if (state.expiresAt && Number.isNaN(Date.parse(state.expiresAt))) {
      return 'Expiração da aprovação é inválida';
    }
    return null;
  };

  const handleLevelChange = (level: AutonomyLevel) => {
    setState((previous) => ({
      ...previous,
      targetLevel: level,
      maxSteps: String(defaultsForLevel(level).max_consecutive_autonomous_steps),
      error: null,
    }));
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!snapshot) return;
    const validationError = validate();
    if (validationError) {
      setState((previous) => ({ ...previous, error: validationError }));
      return;
    }
    if (!hasChanges()) return;

    const policy = defaultsForLevel(state.targetLevel);
    policy.max_consecutive_autonomous_steps = Number(state.maxSteps);
    setState((previous) => ({ ...previous, isSubmitting: true, error: null }));
    try {
      const updated = await apiClient.update({
        project_id: projectId,
        agent_id: agentId,
        policy,
        ...(isEscalation ? {
          approval: {
            approver_id: state.approverId.trim(),
            reason: state.reason.trim(),
            expires_at: state.expiresAt ? new Date(state.expiresAt).toISOString() : null,
          },
        } : {}),
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
      <section className="autonomy-page" aria-label="Política de autonomia do Agent">
        <header className="autonomy-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Política de autonomia do Agent</h1>
        </header>
        <div role="status" aria-live="polite" className="autonomy-page__loading">Carregando política de autonomia...</div>
      </section>
    );
  }

  if (!snapshot) {
    return (
      <section className="autonomy-page" aria-label="Política de autonomia do Agent">
        <header className="autonomy-page__header">
          <button type="button" onClick={onBack} aria-label="Voltar">← Voltar</button>
          <h1>Política de autonomia do Agent</h1>
        </header>
        {state.error ? (
          <div role="alert" className="autonomy-page__error"><p>{state.error}</p><button type="button" onClick={() => void fetchPolicy()}>Tentar novamente</button></div>
        ) : (
          <div role="status" className="autonomy-page__unsupported"><strong>Nenhum serviço de autonomia disponível para este agent.</strong><span>O estado é desconhecido; nenhuma elevação automática será permitida.</span></div>
        )}
      </section>
    );
  }

  return (
    <section className="autonomy-page" aria-label="Política de autonomia do Agent">
      <header className="autonomy-page__header">
        <button type="button" onClick={handleCancel} aria-label="Voltar" disabled={state.isSubmitting}>← Voltar</button>
        <h1>Política de autonomia do Agent</h1>
      </header>

      {state.error && <div role="alert" className="autonomy-page__error"><p>{state.error}</p></div>}

      <div className="autonomy-page__boundary" role="note">
        <strong>Fail-closed · sem autoelevação silenciosa</strong>
        <span>Escalações exigem aprovação humana explícita; o LLM não pode alterar esta política.</span>
      </div>

      <form className="autonomy-page__form" aria-label="Formulário de autonomia" noValidate onSubmit={handleSubmit}>
        <div className="autonomy-page__field">
          <label htmlFor="autonomy-level">Nível de autonomia</label>
          <select id="autonomy-level" value={state.targetLevel} onChange={(event) => handleLevelChange(event.target.value as AutonomyLevel)} disabled={state.isSubmitting}>
            {LEVELS.map((level) => <option key={level} value={level}>{LEVEL_LABELS[level]}</option>)}
          </select>
        </div>

        <div className="autonomy-page__field">
          <label htmlFor="autonomy-max-steps">Máximo de passos autônomos consecutivos</label>
          <input id="autonomy-max-steps" type="number" min="1" max="1000" step="1" value={state.maxSteps} onChange={(event) => setState((previous) => ({ ...previous, maxSteps: event.target.value, error: null }))} disabled={state.isSubmitting} />
        </div>

        {isEscalation && (
          <fieldset className="autonomy-page__approval" disabled={state.isSubmitting}>
            <legend>Aprovação humana para escalação</legend>
            <label>Approver ID<input type="text" value={state.approverId} onChange={(event) => setState((previous) => ({ ...previous, approverId: event.target.value, error: null }))} maxLength={128} /></label>
            <label>Motivo da aprovação<textarea value={state.reason} onChange={(event) => setState((previous) => ({ ...previous, reason: event.target.value, error: null }))} maxLength={256} rows={3} /></label>
            <label>Expiração (opcional)<input type="datetime-local" value={state.expiresAt} onChange={(event) => setState((previous) => ({ ...previous, expiresAt: event.target.value, error: null }))} /></label>
          </fieldset>
        )}

        <section className="autonomy-page__matrix" aria-label="Matriz de decisões de autonomia">
          <h2>{LEVEL_LABELS[state.targetLevel]}</h2>
          <p>Decisões efetivas por operação; não são execução nem aprovação automática.</p>
          <ul>
            {OPERATIONS.map((operation) => <li key={operation}>{operation}: {snapshot.decisions[operation]}</li>)}
          </ul>
        </section>

        <div className="autonomy-page__actions">
          <button type="button" onClick={handleCancel} disabled={state.isSubmitting}>Cancelar</button>
          <button type="submit" disabled={state.isSubmitting || !hasChanges()}>{state.isSubmitting ? 'Salvando política...' : 'Salvar política de autonomia'}</button>
        </div>
      </form>
    </section>
  );
};