import React, { useState, useEffect, useCallback } from 'react';
import { ProjectApiClient, defaultProjectApi } from '../api/projects';
import { AgentApiClient } from '../api/agents';
import { SkillApiClient } from '../api/skills';
import { SkillEditorApiClient } from '../api/skillEditor';
import { ProjectSummary, ProjectStatus } from '../types/project';
import { MemoryPanel } from './MemoryPanel';
import { SkillsPanel } from './SkillsPanel';
import { AutomationList } from './AutomationList';
import { AgentList } from './AgentList';
import './ProjectDetailView.css';

export interface ProjectDetailViewProps {
  projectId: string;
  initialProject?: ProjectSummary;
  apiClient?: ProjectApiClient;
  agentApiClient?: AgentApiClient;
  skillApiClient?: SkillApiClient;
  skillEditorApiClient?: SkillEditorApiClient;
  onBack?: () => void;
  onProjectUpdated?: (project: ProjectSummary) => void;
  onProjectArchived?: (project: ProjectSummary) => void;
}

export const ProjectDetailView: React.FC<ProjectDetailViewProps> = ({
  projectId,
  initialProject,
  apiClient = defaultProjectApi,
  agentApiClient,
  skillApiClient,
  skillEditorApiClient,
  onBack,
  onProjectUpdated,
  onProjectArchived,
}) => {
  const [project, setProject] = useState<ProjectSummary | null>(initialProject ?? null);
  const [isLoading, setIsLoading] = useState<boolean>(!initialProject);
  const [isEditing, setIsEditing] = useState<boolean>(false);
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [isArchiving, setIsArchiving] = useState<boolean>(false);
  const [showArchiveConfirm, setShowArchiveConfirm] = useState<boolean>(false);
  const [archiveReason, setArchiveReason] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'agents'>('overview');

  // Edit form state
  const [editName, setEditName] = useState<string>(initialProject?.name ?? '');
  const [editDescription, setEditDescription] = useState<string>(
    initialProject?.description ?? '',
  );
  const [editStatus, setEditStatus] = useState<ProjectStatus>(
    initialProject?.status ?? 'active',
  );

  const fetchDetail = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const fetched = await apiClient.get(projectId);
      if (fetched) {
        setProject(fetched);
        setEditName(fetched.name);
        setEditDescription(fetched.description ?? '');
        setEditStatus(fetched.status);
      } else if (!initialProject) {
        setError('Projeto não encontrado.');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Falha ao buscar detalhes do projeto.');
    } finally {
      setIsLoading(false);
    }
  }, [apiClient, projectId, initialProject]);

  useEffect(() => {
    if (!initialProject) {
      fetchDetail();
    } else {
      setEditName(initialProject.name);
      setEditDescription(initialProject.description ?? '');
      setEditStatus(initialProject.status);
    }
  }, [fetchDetail, initialProject]);

  const handleStartEditing = () => {
    if (project) {
      setEditName(project.name);
      setEditDescription(project.description ?? '');
      setEditStatus(project.status);
      setError(null);
      setSuccessMsg(null);
      setIsEditing(true);
    }
  };

  const handleSaveUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!project || isSaving) return;

    const trimmedName = editName.trim();
    if (!trimmedName) {
      setError('O nome do projeto é obrigatório.');
      return;
    }
    if (trimmedName.length > 100) {
      setError('O nome do projeto deve ter no máximo 100 caracteres.');
      return;
    }

    setIsSaving(true);
    setError(null);
    setSuccessMsg(null);

    try {
      const response = await apiClient.update({
        id: project.id,
        name: trimmedName,
        description: editDescription.trim() ? editDescription.trim() : null,
        status: editStatus,
        expected_updated_at: project.updated_at,
        correlation_id: `upd_${Date.now().toString(36)}`,
      });

      setProject(response.project);
      setIsEditing(false);
      setSuccessMsg('Projeto atualizado com sucesso.');
      onProjectUpdated?.(response.project);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Falha ao atualizar o projeto.');
    } finally {
      setIsSaving(false);
    }
  };

  const handleConfirmArchive = async () => {
    if (!project || isArchiving) return;

    setIsArchiving(true);
    setError(null);
    setSuccessMsg(null);

    try {
      const response = await apiClient.archive({
        id: project.id,
        reason: archiveReason.trim() ? archiveReason.trim() : undefined,
        correlation_id: `arc_${Date.now().toString(36)}`,
      });

      setProject(response.project);
      setShowArchiveConfirm(false);
      setSuccessMsg('Projeto arquivado com sucesso.');
      onProjectArchived?.(response.project);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Falha ao arquivar o projeto.');
    } finally {
      setIsArchiving(false);
    }
  };

  if (isLoading) {
    return (
      <div className="project-detail-container" aria-label="Detalhes do Projeto">
        <div className="project-state-message" role="status" aria-busy="true">
          Carregando detalhes do projeto...
        </div>
      </div>
    );
  }

  if (!project) {
    return (
      <div className="project-detail-container" aria-label="Detalhes do Projeto">
        {onBack && (
          <button type="button" className="btn-back" onClick={onBack}>
            &larr; Voltar
          </button>
        )}
        <div className="project-state-message project-error" role="alert">
          <p>{error ?? 'Projeto não encontrado.'}</p>
        </div>
      </div>
    );
  }

  const isArchived = project.status === 'archived';

  return (
    <article className="project-detail-container" aria-label={`Detalhes do Projeto ${project.name}`}>
      <header className="project-detail-header">
        <div className="project-detail-header-left">
          {onBack && (
            <button type="button" className="btn-back" onClick={onBack} aria-label="Voltar para a lista">
              &larr; Voltar
            </button>
          )}
          <h2>{project.name}</h2>
          <span className={`project-status-badge ${project.status}`}>{project.status}</span>
        </div>
        <div className="project-detail-actions">
          {!isEditing && !isArchived && (
            <button type="button" className="btn-edit" onClick={handleStartEditing}>
              Editar
            </button>
          )}
          {!isArchived && (
            <button
              type="button"
              className="btn-archive"
              onClick={() => setShowArchiveConfirm(true)}
              aria-label="Arquivar este projeto"
            >
              Arquivar
            </button>
          )}
        </div>
      </header>

      {successMsg && (
        <div className="project-detail-success" role="status">
          {successMsg}
        </div>
      )}

      {error && (
        <div className="project-detail-error" role="alert">
          {error}
        </div>
      )}

      <div className="project-detail-tabs" role="tablist" aria-label="Conteúdo do projeto">
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'overview'}
          onClick={() => setActiveTab('overview')}
        >
          Visão geral
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'agents'}
          onClick={() => setActiveTab('agents')}
        >
          Agents
        </button>
      </div>

      {activeTab === 'agents' ? (
        <AgentList projectId={project.id} apiClient={agentApiClient} />
      ) : isEditing ? (
        <form onSubmit={handleSaveUpdate} className="project-edit-form" noValidate>
          <div className="form-group">
            <label htmlFor="edit-project-name">
              Nome do Projeto <span className="required-star">*</span>
            </label>
            <input
              id="edit-project-name"
              type="text"
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              disabled={isSaving}
              maxLength={100}
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="edit-project-status">Status</label>
            <select
              id="edit-project-status"
              value={editStatus}
              onChange={(e) => setEditStatus(e.target.value as ProjectStatus)}
              disabled={isSaving}
            >
              <option value="active">Ativo (Active)</option>
              <option value="paused">Pausado (Paused)</option>
            </select>
          </div>

          <div className="form-group">
            <label htmlFor="edit-project-desc">Descrição</label>
            <textarea
              id="edit-project-desc"
              value={editDescription}
              onChange={(e) => setEditDescription(e.target.value)}
              disabled={isSaving}
              maxLength={500}
              rows={4}
            />
          </div>

          <div className="form-actions">
            <button
              type="button"
              className="btn-cancel"
              onClick={() => setIsEditing(false)}
              disabled={isSaving}
            >
              Cancelar
            </button>
            <button
              type="submit"
              className="btn-submit"
              disabled={isSaving || !editName.trim()}
            >
              {isSaving ? 'Salvando...' : 'Salvar Alterações'}
            </button>
          </div>
        </form>
      ) : (
        <div className="project-detail-body">
          <section className="detail-section">
            <h3>Informações Gerais</h3>
            <dl className="detail-grid">
              <dt>ID do Projeto:</dt>
              <dd className="monospace">{project.id}</dd>

              <dt>Responsável:</dt>
              <dd>{project.owner}</dd>

              <dt>Descrição:</dt>
              <dd>{project.description || 'Sem descrição cadastrada.'}</dd>

              <dt>Criado em:</dt>
              <dd>{new Date(project.created_at).toLocaleString()}</dd>

              <dt>Última atualização:</dt>
              <dd>{new Date(project.updated_at).toLocaleString()}</dd>
            </dl>
          </section>

          {project.settings && (
            <section className="detail-section">
              <h3>Configurações do Projeto</h3>
              <dl className="detail-grid">
                <dt>Retenção de dados:</dt>
                <dd>{`${project.settings.retention_days} dias`}</dd>

                <dt>Máximo de agentes ativos:</dt>
                <dd>{project.settings.max_active_agents}</dd>


                <dt>Telemetria:</dt>
                <dd>{project.settings.telemetry_enabled ? 'Habilitada' : 'Desabilitada'}</dd>

                <dt>Auto-arquivamento por inatividade:</dt>
                <dd>
                  {project.settings.auto_archive_idle_days
                    ? `${project.settings.auto_archive_idle_days} dias`
                    : 'Desativado'}
                </dd>
              </dl>
            </section>
          )}
        </div>
      )}

      <MemoryPanel projectId={project.id} />
      <SkillsPanel projectId={project.id} apiClient={skillApiClient} skillEditorApiClient={skillEditorApiClient} />
      <AutomationList projectId={project.id} ownerId={project.owner} />

      {showArchiveConfirm && (
        <div
          className="modal-overlay"
          role="dialog"
          aria-modal="true"
          aria-labelledby="archive-dialog-title"
        >
          <div className="modal-content">
            <h3 id="archive-dialog-title">Confirmar Arquivamento</h3>
            <p>
              Tem certeza de que deseja arquivar o projeto <strong>{project.name}</strong>?
              Esta ação desativa os workflows em execução.
            </p>
            <div className="form-group">
              <label htmlFor="archive-reason-input">Motivo do arquivamento (opcional):</label>
              <input
                id="archive-reason-input"
                type="text"
                value={archiveReason}
                onChange={(e) => setArchiveReason(e.target.value)}
                placeholder="Ex: Projeto concluído"
                disabled={isArchiving}
                maxLength={200}
              />
            </div>
            <div className="modal-actions">
              <button
                type="button"
                className="btn-cancel"
                onClick={() => setShowArchiveConfirm(false)}
                disabled={isArchiving}
              >
                Cancelar
              </button>
              <button
                type="button"
                className="btn-danger"
                onClick={handleConfirmArchive}
                disabled={isArchiving}
              >
                {isArchiving ? 'Arquivando...' : 'Confirmar Arquivamento'}
              </button>
            </div>
          </div>
        </div>
      )}
    </article>
  );
};
