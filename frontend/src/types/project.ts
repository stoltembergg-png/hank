/**
 * Contratos de tipo para gerenciamento de Projects no frontend.
 * Conforme PR-036, PR-037, PR-038 e PR-029/PR-030/PR-031/PR-032/PR-035.
 */

export type ProjectStatus = 'active' | 'paused' | 'archived';

export interface ProjectSettingsSummary {
  retention_days: number;
  auto_archive_idle_days?: number | null;
  telemetry_enabled: boolean;
  max_active_agents: number;
}

export interface ProjectSummary {
  id: string;
  name: string;
  description?: string | null;
  status: ProjectStatus;
  owner: string;
  created_at: string;
  updated_at: string;
  settings?: ProjectSettingsSummary;
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

export interface UpdateProjectInput {
  id: string;
  name?: string;
  description?: string | null;
  status?: ProjectStatus;
  expected_updated_at?: string;
  correlation_id?: string | null;
}

export interface UpdateProjectOutput {
  project: ProjectSummary;
  event_id?: string | null;
  correlation_id?: string | null;
}

export interface ArchiveProjectInput {
  id: string;
  reason?: string;
  correlation_id?: string | null;
}

export interface ArchiveProjectOutput {
  project: ProjectSummary;
  event_id?: string | null;
  already_archived: boolean;
  correlation_id?: string | null;
}
