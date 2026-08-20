/**
 * Contratos de tipo para gerenciamento de Projects no frontend.
 * Conforme PR-036 e PR-030 (ListProjectsService).
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
