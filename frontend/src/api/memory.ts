import { ListMemoriesInput, ListMemoriesOutput } from '../types/memory';

export interface MemoryApiClient {
  list(input: ListMemoriesInput): Promise<ListMemoriesOutput>;
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

export class DesktopMemoryApiClient implements MemoryApiClient {
  async list(input: ListMemoriesInput): Promise<ListMemoriesOutput> {
    const projectId = input.project_id.trim();
    if (!projectId || projectId.length > 128 || hasControlCharacters(projectId)) {
      throw new Error('Identidade do projeto inválida.');
    }

    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ListMemoriesOutput>('list_memories', { input: { ...input, project_id: projectId } });
      }
    }

    return { project_id: projectId, memories: [] };
  }
}

function hasControlCharacters(value: string): boolean {
  return Array.from(value).some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code <= 0x1f || code === 0x7f;
  });
}

export const defaultMemoryApi = new DesktopMemoryApiClient();
