import React, { useState, useEffect, useCallback } from 'react';
import { ProjectApiClient, defaultProjectApi } from '../api/projects';
import { ProjectSummary, ProjectStatus } from '../types/project';
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

  const currentPage = Math.floor(offset / pageSize) + 1;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));

  return (
    <section className="project-list-container" aria-label="Gerenciamento de Projetos">
      <header className="project-list-header">
        <h2>Projetos ({total})</h2>
      </header>

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
              <li key={project.id} className="project-card" role="listitem">
                <div className="project-card-main">
                  <span className="project-card-title">{project.name}</span>
                  {project.description && (
                    <span className="project-card-desc">{project.description}</span>
                  )}
                  <span className="project-card-meta">
                    Responsável: {project.owner} | Criado em:{' '}
                    {new Date(project.created_at).toLocaleDateString()}
                  </span>
                </div>
                <span className={`project-status-badge ${project.status}`}>
                  {project.status}
                </span>
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
