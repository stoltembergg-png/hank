/**
 * Cliente de API desacoplado para serviços de Project.
 * Conforme PR-036, PR-037, PR-038 e fronteiras de isolamento.
 */

import {
  ArchiveProjectInput,
  ArchiveProjectOutput,
  CreateProjectInput,
  CreateProjectOutput,
  ListProjectsInput,
  ListProjectsOutput,
  ProjectSummary,
  UpdateProjectInput,
  UpdateProjectOutput,
} from '../types/project';

export interface ProjectApiClient {
  list(input?: ListProjectsInput): Promise<ListProjectsOutput>;
  get(id: string): Promise<ProjectSummary | null>;
  create(input: CreateProjectInput): Promise<CreateProjectOutput>;
  update(input: UpdateProjectInput): Promise<UpdateProjectOutput>;
  archive(input: ArchiveProjectInput): Promise<ArchiveProjectOutput>;
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

export class DesktopProjectApiClient implements ProjectApiClient {
  async list(input: ListProjectsInput = {}): Promise<ListProjectsOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ListProjectsOutput>('list_projects', { input });
      }
    }

    // Retorno fallback seguro para ambiente desacoplado, teste ou browser
    return {
      projects: [],
      total: 0,
      limit: Math.min(Math.max(input.limit ?? 20, 1), 100),
      offset: Math.max(input.offset ?? 0, 0),
    };
  }

  async get(id: string): Promise<ProjectSummary | null> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ProjectSummary | null>('get_project', { id });
      }
    }

    return null;
  }

  async create(input: CreateProjectInput): Promise<CreateProjectOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<CreateProjectOutput>('create_project', { input });
      }
    }

    // Retorno fallback seguro para ambiente desacoplado / teste / demo
    const now = new Date().toISOString();
    return {
      project: {
        id: `prj_${Date.now().toString(36)}`,
        name: input.name.trim(),
        owner: input.owner.trim(),
        description: input.description?.trim() || null,
        status: 'active',
        created_at: now,
        updated_at: now,
        settings: {
          retention_days: 90,
          auto_archive_idle_days: null,
          telemetry_enabled: false,
          max_active_agents: 5,
        },
      },
      correlation_id: input.correlation_id ?? null,
    };
  }

  async update(input: UpdateProjectInput): Promise<UpdateProjectOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<UpdateProjectOutput>('update_project', { input });
      }
    }

    const now = new Date().toISOString();
    return {
      project: {
        id: input.id,
        name: input.name ?? 'Updated Project',
        description: input.description ?? null,
        status: input.status ?? 'active',
        owner: 'current_owner',
        created_at: input.expected_updated_at ?? now,
        updated_at: now,
      },
      correlation_id: input.correlation_id ?? null,
    };
  }

  async archive(input: ArchiveProjectInput): Promise<ArchiveProjectOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ArchiveProjectOutput>('archive_project', { input });
      }
    }

    const now = new Date().toISOString();
    return {
      project: {
        id: input.id,
        name: 'Archived Project',
        description: null,
        status: 'archived',
        owner: 'current_owner',
        created_at: now,
        updated_at: now,
      },
      already_archived: false,
      correlation_id: input.correlation_id ?? null,
    };
  }
}

export const defaultProjectApi = new DesktopProjectApiClient();
