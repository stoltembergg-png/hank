/**
 * Cliente de API desacoplado para serviços de Project.
 * Conforme PR-036 e fronteiras de isolamento.
 */

import { ListProjectsInput, ListProjectsOutput } from '../types/project';

export interface ProjectApiClient {
  list(input?: ListProjectsInput): Promise<ListProjectsOutput>;
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
}

export const defaultProjectApi = new DesktopProjectApiClient();
