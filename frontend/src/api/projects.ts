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

export class ProjectBridgeUnavailableError extends Error {
  readonly code = 'PROJECT_BRIDGE_UNAVAILABLE';

  constructor() {
    super('Project desktop bridge is unavailable; no synthetic fallback is permitted');
    this.name = 'ProjectBridgeUnavailableError';
  }
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

    throw new ProjectBridgeUnavailableError();
  }

  async get(id: string): Promise<ProjectSummary | null> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ProjectSummary | null>('get_project', { id });
      }
    }

    throw new ProjectBridgeUnavailableError();
  }

  async create(input: CreateProjectInput): Promise<CreateProjectOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<CreateProjectOutput>('create_project', { input });
      }
    }

    throw new ProjectBridgeUnavailableError();
  }

  async update(input: UpdateProjectInput): Promise<UpdateProjectOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<UpdateProjectOutput>('update_project', { input });
      }
    }

    throw new ProjectBridgeUnavailableError();
  }

  async archive(input: ArchiveProjectInput): Promise<ArchiveProjectOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ArchiveProjectOutput>('archive_project', { input });
      }
    }

    throw new ProjectBridgeUnavailableError();
  }
}

export const defaultProjectApi = new DesktopProjectApiClient();
