/**
 * Cliente de API desacoplado para serviços de Agent.
 * Conforme PR-048, PR-049 e fronteiras de isolamento.
 */

import {
  ArchiveAgentInput,
  ArchiveAgentOutput,
  CreateAgentInput,
  CreateAgentOutput,
  ListAgentsInput,
  ListAgentsOutput,
  UpdateAgentInput,
  UpdateAgentOutput,
  AgentSummary,
} from '../types/agent';

export interface AgentApiClient {
  list(input: ListAgentsInput): Promise<ListAgentsOutput>;
  get(projectId: string, agentId: string): Promise<AgentSummary | null>;
  create(input: CreateAgentInput): Promise<CreateAgentOutput>;
  update(input: UpdateAgentInput): Promise<UpdateAgentOutput>;
  archive(input: ArchiveAgentInput): Promise<ArchiveAgentOutput>;
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

export class DesktopAgentApiClient implements AgentApiClient {
  async list(input: ListAgentsInput): Promise<ListAgentsOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ListAgentsOutput>('list_agents', { input });
      }
    }

    // Retorno fallback seguro para ambiente desacoplado, teste ou browser
    return {
      agents: [],
      total: 0,
      limit: Math.min(Math.max(input.limit ?? 20, 1), 100),
      offset: Math.max(input.offset ?? 0, 0),
    };
  }

  async get(projectId: string, agentId: string): Promise<AgentSummary | null> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<AgentSummary | null>('get_agent', { projectId, agentId });
      }
    }

    return null;
  }

  async create(input: CreateAgentInput): Promise<CreateAgentOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<CreateAgentOutput>('create_agent', { input });
      }
    }

    // Retorno fallback seguro para ambiente desacoplado / teste / demo
    const now = new Date().toISOString();
    return {
      agent: {
        id: `agt_${Date.now().toString(36)}`,
        project_id: input.project_id,
        name: input.name.trim(),
        description: input.description?.trim() || null,
        status: 'active',
        personality: {
          name: 'Default',
          description: null,
          traits: ['helpful', 'accurate'],
          communication_style: 'technical',
        },
        created_at: now,
        updated_at: now,
      },
      event_id: `evt_${Date.now().toString(36)}`,
      correlation_id: input.correlation_id || null,
    };
  }

  async update(input: UpdateAgentInput): Promise<UpdateAgentOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<UpdateAgentOutput>('update_agent', { input });
      }
    }

    // Fallback com erro de versão para simular optimistic locking
    throw new Error('Optimistic version check failed - stale version');
  }

  async archive(input: ArchiveAgentInput): Promise<ArchiveAgentOutput> {
    if (typeof window !== 'undefined') {
      const bridgeWin = window as unknown as InjectedBridgeWindow;
      const invoker = bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
      if (typeof invoker === 'function') {
        return await invoker<ArchiveAgentOutput>('archive_agent', { input });
      }
    }

    // Fallback
    throw new Error('Archive requires explicit confirmation');
  }
}

export const defaultAgentApi = new DesktopAgentApiClient();