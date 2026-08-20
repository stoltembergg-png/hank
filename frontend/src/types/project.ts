/**
 * Contratos de tipo para gerenciamento de Projects no frontend.
 * Conforme PR-036, PR-037 e PR-030/PR-029 (ListProjectsService, CreateProjectService).
 */

export type ProjectStatus = 'active' | 'paused' | 'archived';

export interface ProjectSummary {
  id: string;
  name: string;
  description?: string | null;
  status: ProjectStatus;
  owner: string;
  created_at: string;
  updated_at: string;
}

export interface ListProjectsInput {
  limit?: number;
  offset?: number;
  status?: ProjectStatus;
}

export interface ListProjectsOutput {
  projects: ProjectSummary[];
  total: number;
  limit: number;
  offset: number;
}

export interface CreateProjectInput {
  name: string;
  owner: string;
  description?: string | null;
  correlation_id?: string | null;
}

export interface CreateProjectOutput {
  project: ProjectSummary;
  event_id?: string | null;
  correlation_id?: string | null;
}
