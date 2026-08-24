import React, { useCallback, useEffect, useMemo, useState } from 'react';
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

  const fetchSkills = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    setMutationError(null);
    try {
      const result = await apiClient.list({ project_id: projectId, scope, limit: 50 });
      validateProjectResponse(result, projectId, scope);
      setSkills(result.skills.slice(0, 50));
      setAvailable(result.available !== false);
    } catch (reason) {
      setSkills([]);
      setAvailable(true);
      setError(reason instanceof Error ? reason.message : 'Falha ao carregar skills.');
    } finally {
      setIsLoading(false);
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
    if (nextScope !== scope) setScope(nextScope);
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
              mutating={mutatingSkillId === skill.id}
              onRollback={() => void rollback(skill)}
            />
          ))}
        </ul>
      )}
    </section>
  );
};

function SkillCard({
  skill,
  mutating,
  onRollback,
}: {
  skill: SkillSummary;
  mutating: boolean;
  onRollback: () => void;
}) {
  const isGlobalUnavailable = skill.scope === 'global' && (!skill.binding || !skill.binding.enabled);
  const capabilities = skill.capabilities.map((capability) => (
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
        <dt>Policy</dt><dd>{skill.policy.requires_approval ? 'Aprovação obrigatória' : 'Aprovação não exigida'}</dd>
        <dt>Budget</dt><dd>{skill.budget.max_tokens.toLocaleString()} tokens · {skill.budget.max_parallel_invocations} paralelas</dd>
        <dt>Trace</dt><dd>{skill.binding?.trace_id ?? skill.trace_id}</dd>
        {skill.binding && <><dt>Binding</dt><dd>{skill.binding.enabled ? 'Ativo' : 'Desabilitado'} · revisão {skill.binding.revision}</dd></>}
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

      {skill.scope === 'project' && skill.binding?.enabled && (
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
  const wrongScope = result.skills.some((skill) => skill.scope !== scope);
  const wrongProject = result.skills.some((skill) => (
    scope === 'project' ? skill.project_id !== projectId : skill.project_id !== null
  ));
  if (wrongScope || wrongProject) {
    throw new Error('Resposta de skills fora do projeto selecionado.');
  }
}

function safeDescription(description: string): string {
  const bounded = description.slice(0, MAX_DESCRIPTION_CHARS);
  return bounded.length < description.length ? `${bounded}…` : bounded;
}

function shortDigest(digest: string): string {
  return digest.length > 16 ? `${digest.slice(0, 16)}…` : digest;
}

function skillMutationError(error: unknown): string {
  const message = error instanceof Error ? error.message : '';
  if (/concurrency|stale|version/i.test(message)) {
    return 'Conflito de versão: a skill mudou antes da confirmação. Recarregue e tente novamente.';
  }
  return 'O rollback da skill foi rejeitado sem alterar o vínculo.';
}
