import { ListMemoriesInput, ListMemoriesOutput, MemorySummary } from '../types/memory';

export type MemoryMutationEdit =
  | { kind: 'update'; content: string; summary?: string | null; importance: number }
  | { kind: 'approve' }
  | { kind: 'reject' }
  | { kind: 'archive' }
  | { kind: 'restore' };

export interface MemoryMutationInput {
  project_id: string;
  agent_id?: string;
  memory_id: string;
  actor_id: string;
  trace_id: string;
  operation_id: string;
  capability: 'memory.write';
  expected_version: number;
  confirmed: boolean;
  edit: MemoryMutationEdit;
}

export interface MemoryApiClient {
  list(input: ListMemoriesInput): Promise<ListMemoriesOutput>;
  mutate?(input: MemoryMutationInput): Promise<MemorySummary>;
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

  async mutate(input: MemoryMutationInput): Promise<MemorySummary> {
    const normalized = normalizeMutationInput(input);
    const invoker = bridgeInvoker();
    if (typeof invoker !== 'function') {
      throw new Error('Serviço de edição de memória indisponível.');
    }
    return await invoker<MemorySummary>('mutate_memory', { input: normalized });
  }
}

function bridgeInvoker() {
  if (typeof window === 'undefined') return undefined;
  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

function normalizeMutationInput(input: MemoryMutationInput): MemoryMutationInput {
  for (const value of [input.project_id, input.memory_id, input.actor_id, input.trace_id, input.operation_id]) {
    if (!isBoundedIdentifier(value)) {
      throw new Error('Contexto de edição de memória inválido.');
    }
  }
  if (input.capability !== 'memory.write' || input.confirmed !== true || !Number.isSafeInteger(input.expected_version)) {
    throw new Error('Contexto de edição de memória inválido.');
  }
  if (input.edit.kind === 'update' && (!input.edit.content.trim() || input.edit.content.length > 16 * 1024)) {
    throw new Error('Conteúdo de memória inválido ou excede o limite.');
  }
  return input;
}

function isBoundedIdentifier(value: string): boolean {
  return value.trim().length > 0 && value.length <= 128 && !hasControlCharacters(value);
}

function hasControlCharacters(value: string): boolean {
  return Array.from(value).some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code <= 0x1f || code === 0x7f;
  });
}

export const defaultMemoryApi = new DesktopMemoryApiClient();
