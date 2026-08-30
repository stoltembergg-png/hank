import React, { useState, useEffect, useCallback } from 'react';
import { ProjectApiClient, defaultProjectApi } from '../api/projects';
import { ProjectSummary, ProjectStatus } from '../types/project';
import { CreateProjectForm } from './CreateProjectForm';
import { ProjectDetailView } from './ProjectDetailView';
import './ProjectList.css';

export interface ProjectListProps {
  apiClient?: ProjectApiClient;
  statusFilter?: ProjectStatus;
  pageSize?: number;
}

export const ProjectList: React.FC<ProjectListProps> = ({
  apiClient = defaultProjectApi,
  statusFilter,
  pageSize = 10,
}) => {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [total, setTotal] = useState<number>(0);
  const [offset, setOffset] = useState<number>(0);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState<boolean>(false);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);

  const fetchProjects = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await apiClient.list({
        limit: pageSize,
        offset,
        status: statusFilter,
      });
      setProjects(response.projects);
      setTotal(response.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Falha ao carregar projetos');
    } finally {
      setIsLoading(false);
    }
  }, [apiClient, pageSize, offset, statusFilter]);

  useEffect(() => {
    fetchProjects();
  }, [fetchProjects]);

  const handleNextPage = () => {
    if (offset + pageSize < total) {
      setOffset((prev) => prev + pageSize);
    }
  };

  const handlePrevPage = () => {
    setOffset((prev) => Math.max(0, prev - pageSize));
  };

  const handleProjectCreated = () => {
    setIsCreating(false);
    setOffset(0);
    fetchProjects();
  };

  const handleProjectUpdated = (updated: ProjectSummary) => {
    setProjects((prev) =>
      prev.map((p) => (p.id === updated.id ? updated : p)),
    );
  };

  const handleProjectArchived = (archived: ProjectSummary) => {
    setProjects((prev) =>
      prev.map((p) => (p.id === archived.id ? archived : p)),
    );
  };

  if (selectedProjectId) {
    const selected = projects.find((p) => p.id === selectedProjectId);
    return (
      <ProjectDetailView
        projectId={selectedProjectId}
        initialProject={selected}
        apiClient={apiClient}
        onBack={() => setSelectedProjectId(null)}
        onProjectUpdated={handleProjectUpdated}
        onProjectArchived={handleProjectArchived}
      />
    );
  }

  const currentPage = Math.floor(offset / pageSize) + 1;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  return (
    <section className="project-list-container" aria-label="Gerenciamento de Projetos">
      <header className="project-list-header">
        <div className="project-list-heading">
          <span className="project-list-eyebrow">Workspace</span>
          <h2>Projetos</h2>
          <p className="project-list-description">Organize seus projetos e sessões.</p>
        </div>
        <div className="project-list-actions" role="toolbar" aria-label="Ferramentas de projetos">
          <span className="project-list-count">
            {total} {total === 1 ? 'projeto' : 'projetos'}
          </span>
          {!isCreating && (
            <button
              type="button"
              className="btn-new-project"
              onClick={() => setIsCreating(true)}
              aria-label="Abrir formulário de criação de projeto"
            >
              <span className="btn-new-project-icon" aria-hidden="true">＋</span>
              <span>Novo Projeto</span>
            </button>
          )}
        </div>
      </header>

      {isCreating && (
        <CreateProjectForm
          apiClient={apiClient}
          onSuccess={handleProjectCreated}
          onCancel={() => setIsCreating(false)}
        />
      )}

      {isLoading && (
        <div className="project-state-message" role="status" aria-busy="true">
          Carregando projetos...
        </div>
      )}

      {!isLoading && error && (
        <div className="project-state-message project-error" role="alert">
          <p>{error}</p>
          <button type="button" className="btn-retry" onClick={fetchProjects}>
            Tentar novamente
          </button>
        </div>
      )}

      {!isLoading && !error && projects.length === 0 && (
        <div className="project-state-message" role="status">
          Nenhum projeto encontrado.
        </div>
      )}

      {!isLoading && !error && projects.length > 0 && (
        <>
          <ul className="project-list" role="list">
            {projects.map((project) => (
              <li
                key={project.id}
                className="project-card clickable"
                role="listitem"
                onClick={() => setSelectedProjectId(project.id)}
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    setSelectedProjectId(project.id);
                  }
                }}
                aria-label={`Ver detalhes de ${project.name}`}
              >
                <span className="project-card-icon" aria-hidden="true">◇</span>
                <div className="project-card-main">
                  <div className="project-card-title-row">
                    <span className="project-card-title">{project.name}</span>
                    <span className={`project-status-badge ${project.status}`}>
                      {project.status}
                    </span>
                  </div>
                  {project.description && (
                    <span className="project-card-desc">{project.description}</span>
                  )}
                  <span className="project-card-meta">
                    Responsável: {project.owner} | Criado em:{' '}
                    {new Date(project.created_at).toLocaleDateString()}
                  </span>
                </div>
                <span className="project-card-open" aria-hidden="true">→</span>
              </li>
            ))}
          </ul>

          <nav className="project-pagination" aria-label="Paginação de projetos">
            <button
              type="button"
              onClick={handlePrevPage}
              disabled={offset === 0 || isLoading}
              aria-label="Página anterior"
            >
              Anterior
            </button>
            <span aria-current="page">
              Página {currentPage} de {totalPages}
            </span>
            <button
              type="button"
              onClick={handleNextPage}
              disabled={offset + pageSize >= total || isLoading}
              aria-label="Próxima página"
            >
              Próxima
            </button>
          </nav>
        </>
      )}
    </section>
  );
};
