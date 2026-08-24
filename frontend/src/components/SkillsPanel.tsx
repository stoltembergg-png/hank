import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { defaultSkillApi, SkillApiClient } from '../api/skills';
import {
  SkillListOutput,
  SkillScope,
  SkillSummary,
} from '../types/skill';
import './SkillsPanel.css';

const MAX_DESCRIPTION_CHARS = 320;

export interface SkillsPanelProps {
  projectId: string;
  actorId?: string;
  apiClient?: SkillApiClient;
}

export const SkillsPanel: React.FC<SkillsPanelProps> = ({
  projectId,
  actorId = 'desktop-operator',
  apiClient = defaultSkillApi,
}) => {
  const [scope, setScope] = useState<SkillScope>('project');
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [available, setAvailable] = useState(true);
  const [query, setQuery] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [mutationError, setMutationError] = useState<string | null>(null);
  const [mutatingSkillId, setMutatingSkillId] = useState<string | null>(null);
  const requestSequence = useRef(0);

  const fetchSkills = useCallback(async () => {
    const requestId = ++requestSequence.current;
    setIsLoading(true);
    setError(null);
    setMutationError(null);
    try {
      const result = await apiClient.list({ project_id: projectId, scope, limit: 50 });
      validateProjectResponse(result, projectId, scope);
      if (requestId !== requestSequence.current) return;
      setSkills(result.skills.slice(0, 50));
      setAvailable(result.available !== false);
    } catch (reason) {
      if (requestId !== requestSequence.current) return;
      setSkills([]);
      setAvailable(true);
      setError(reason instanceof Error ? reason.message : 'Falha ao carregar skills.');
    } finally {
      if (requestId === requestSequence.current) setIsLoading(false);
    }
  }, [apiClient, projectId, scope]);

  useEffect(() => {
    void fetchSkills();
  }, [fetchSkills]);

  const visibleSkills = useMemo(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase('en-US');
    if (!normalizedQuery) return skills;
    return skills.filter((skill) => (
      `${skill.name} ${skill.id} ${skill.description}`.toLocaleLowerCase('en-US').includes(normalizedQuery)
    ));
  }, [query, skills]);

  const changeScope = (nextScope: SkillScope) => {
    if (nextScope !== scope) {
      setSkills([]);
      setError(null);
      setAvailable(true);
      setScope(nextScope);
    }
  };

  const rollback = async (skill: SkillSummary) => {
    if (!apiClient.rollback || !skill.binding || !skill.binding.enabled || mutatingSkillId) return;
    if (!window.confirm(`Rollback da skill ${skill.name}? O vínculo ativo será desfeito.`)) return;

    setMutatingSkillId(skill.id);
    setMutationError(null);
    try {
      const updated = await apiClient.rollback({
        project_id: projectId,
        skill_id: skill.id,
        actor_id: actorId,
        trace_id: skill.binding.trace_id || skill.trace_id,
        expected_revision: skill.binding.revision,
        approval_id: skill.binding.approval_id,
        capability: 'skill.rollback',
        confirmed: true,
      });
      if (updated.project_id !== projectId) {
        throw new Error('Resposta de rollback fora do projeto selecionado.');
      }
      await fetchSkills();
    } catch (reason) {
      setMutationError(skillMutationError(reason));
    } finally {
      setMutatingSkillId(null);
    }
  };

  return (
    <section className="skills-panel" aria-label="Skills do projeto">
      <header className="skills-panel-header">
        <div>
          <h3>Skills</h3>
          <p className="skills-panel-scope">Projeto: <code>{projectId}</code></p>
        </div>
        <button type="button" onClick={() => void fetchSkills()} disabled={isLoading}>
          Atualizar
        </button>
      </header>

      <div className="skills-panel-tabs" role="tablist" aria-label="Escopo das skills">
        <button type="button" role="tab" aria-selected={scope === 'project'} onClick={() => changeScope('project')}>
          Do projeto
        </button>
        <button type="button" role="tab" aria-selected={scope === 'global'} onClick={() => changeScope('global')}>
          Globais
        </button>
      </div>

      <label className="skills-search">
        Buscar skills
        <input
          type="search"
          aria-label="Buscar skills"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Nome, ID ou descrição"
        />
      </label>

      {isLoading && <p role="status" aria-busy="true">Carregando skills...</p>}
      {!isLoading && error && <p role="alert">{error}</p>}
      {mutationError && <p className="skills-mutation-error" role="alert">{mutationError}</p>}
      {!isLoading && !error && !available && (
        <p role="status">Serviço de skills indisponível neste desktop.</p>
      )}
      {!isLoading && !error && available && skills.length === 0 && (
        <p role="status">Nenhuma skill encontrada neste escopo.</p>
      )}
      {!isLoading && !error && available && skills.length > 0 && visibleSkills.length === 0 && (
        <p role="status">Nenhuma skill corresponde à busca.</p>
      )}
      {!isLoading && !error && available && visibleSkills.length > 0 && (
        <ul className="skills-list" role="list">
          {visibleSkills.map((skill) => (
            <SkillCard
              key={skill.id}
              skill={skill}
              projectId={projectId}
              mutating={mutatingSkillId === skill.id}
              onRollback={apiClient.rollback ? () => void rollback(skill) : undefined}
            />
          ))}
        </ul>
      )}
    </section>
  );
};

function SkillCard({
  skill,
  projectId,
  mutating,
  onRollback,
}: {
  skill: SkillSummary;
  projectId: string;
  mutating: boolean;
  onRollback?: () => void;
}) {
  const bindingAvailable = isBindingAvailable(skill, projectId);
  const isGlobalUnavailable = skill.scope === 'global' && !bindingAvailable;
  const capabilities = skill.capabilities.slice(0, 32).map((capability) => (
    `${capability.resource}:${capability.action}${capability.scope ? ` (${capability.scope})` : ''}`
  ));
  const versions = skill.versions.slice(0, 20);

  return (
    <li className="skill-card">
      <div className="skill-card-header">
        <div>
          <h4>{skill.name}</h4>
          <p className="skill-description">{safeDescription(skill.description)}</p>
        </div>
        <span className={`skill-status ${skill.status}`}>{skill.status}</span>
      </div>

      <p className="skill-scope-badge">{skill.scope} · {skill.source.kind}</p>
      {isGlobalUnavailable && (
        <p className="skill-unavailable" role="status">Indisponível: importe explicitamente para este projeto.</p>
      )}

      <dl className="skill-metadata">
        <dt>Versão ativa</dt><dd>{skill.pinned_version ?? 'não fixada'}</dd>
        <dt>Versão exibida</dt><dd>{skill.version}</dd>
        <dt>Compatibilidade</dt><dd>{skill.compatibility}</dd>
        <dt>Proveniência</dt><dd>source:{skill.source.kind} · digest:{shortDigest(skill.source.reference_digest)}</dd>
        <dt>Policy</dt><dd>{skill.policy.requires_approval ? 'Aprovação obrigatória' : 'Aprovação não exigida'}{skill.binding?.approval_id ? ` · ${safeMetadata(skill.binding.approval_id)}` : ''}</dd>
        <dt>Budget</dt><dd>{skill.budget.max_tokens.toLocaleString('pt-BR')} tokens · {skill.budget.max_cost_micro_usd.toLocaleString('pt-BR')} micro-USD · {skill.budget.max_parallel_invocations} paralelas · {skill.budget.max_wall_time_seconds}s · {resetPeriodLabel(skill.budget.reset_period)}</dd>
        <dt>Trace</dt><dd>{skill.binding?.trace_id ?? skill.trace_id}</dd>
        {skill.binding && <><dt>Binding</dt><dd>{skill.binding.enabled ? 'Ativo' : 'Desabilitado'} · revisão {skill.binding.revision} · versão {skill.binding.current_version}</dd></>}
        {skill.scope === 'global' && bindingAvailable && <><dt>Importação</dt><dd>Importação explícita · versão {skill.binding?.current_version}</dd></>}
      </dl>

      {capabilities.length > 0 && (
        <p className="skill-capabilities"><strong>Capabilities:</strong> {capabilities.join(', ')}</p>
      )}

      <details className="skill-history">
        <summary>Histórico de versões ({versions.length})</summary>
        {versions.length > 0 ? (
          <ul>
            {versions.map((version) => (
              <li key={version.version}>
                <strong>{version.version}</strong> · {version.status} · {version.compatibility}
              </li>
            ))}
          </ul>
        ) : <p>Nenhuma versão histórica disponível.</p>}
      </details>

      {skill.scope === 'project' && bindingAvailable && skill.binding?.enabled && onRollback && (
        <div className="skill-card-actions">
          <button type="button" onClick={onRollback} disabled={mutating} aria-label={`Rollback ${skill.name}`}>
            {mutating ? 'Revertendo...' : 'Rollback'}
          </button>
        </div>
      )}
    </li>
  );
}

function validateProjectResponse(result: SkillListOutput, projectId: string, scope: SkillScope) {
  if (result.project_id !== projectId || result.scope !== scope) {
    throw new Error('Resposta de skills fora do projeto selecionado.');
  }
  const invalidSkill = result.skills.some((skill) => (
    skill.scope !== scope
    || (scope === 'project' && skill.project_id !== projectId)
    || (scope === 'global' && skill.project_id !== null)
    || (skill.binding !== null && !isBindingShapeValid(skill, projectId))
  ));
  if (invalidSkill) {
    throw new Error('Resposta de skills fora do projeto selecionado.');
  }
}

function isBindingShapeValid(skill: SkillSummary, projectId: string): boolean {
  const binding = skill.binding;
  if (!binding || binding.project_id !== projectId || binding.scope !== skill.scope) return false;
  if (!binding.enabled) return true;
  if (binding.current_version !== (skill.pinned_version ?? skill.version)) return false;
  if (skill.scope === 'global') {
    return binding.import_reference?.startsWith('project-import:') === true
      && (!skill.policy.requires_approval || Boolean(binding.approval_id));
  }
  return binding.import_reference === null;
}

function isBindingAvailable(skill: SkillSummary, projectId: string): boolean {
  return Boolean(skill.binding?.enabled && isBindingShapeValid(skill, projectId));
}

function safeDescription(description: string): string {
  const bounded = description.slice(0, MAX_DESCRIPTION_CHARS);
  return bounded.length < description.length ? `${bounded}…` : bounded;
}

function shortDigest(digest: string): string {
  if (!/^[a-f0-9]{64}$/i.test(digest)) return 'digest indisponível';
  return `${digest.slice(0, 16)}…`;
}

function safeMetadata(value: string): string {
  const hasControl = Array.from(value).some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code <= 0x1f || code === 0x7f;
  });
  return value.length <= 128 && !hasControl ? value : 'indisponível';
}

function resetPeriodLabel(period: string): string {
  return ({ never: 'nunca', daily: 'diário', weekly: 'semanal', monthly: 'mensal' } as Record<string, string>)[period] ?? 'indisponível';
}

function skillMutationError(error: unknown): string {
  const message = error instanceof Error ? error.message : '';
  if (/concurrency|stale|version/i.test(message)) {
    return 'Conflito de versão: a skill mudou antes da confirmação. Recarregue e tente novamente.';
  }
  return 'O rollback da skill foi rejeitado sem alterar o vínculo.';
}
