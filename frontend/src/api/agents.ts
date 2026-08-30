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

export const AGENT_BRIDGE_UNAVAILABLE_CODE = 'AGENT_BRIDGE_UNAVAILABLE' as const;

export class AgentBridgeUnavailableError extends Error {
  readonly code = AGENT_BRIDGE_UNAVAILABLE_CODE;

  constructor() {
    super('Agent desktop bridge is unavailable');
    this.name = 'AgentBridgeUnavailableError';
  }
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

function getAgentInvoker() {
  if (typeof window === 'undefined') return undefined;

  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

export class DesktopAgentApiClient implements AgentApiClient {
  async list(input: ListAgentsInput): Promise<ListAgentsOutput> {
    const invoker = getAgentInvoker();
    if (typeof invoker === 'function') {
      return await invoker<ListAgentsOutput>('list_agents', { input });
    }

    throw new AgentBridgeUnavailableError();
  }

  async get(projectId: string, agentId: string): Promise<AgentSummary | null> {
    const invoker = getAgentInvoker();
    if (typeof invoker === 'function') {
      return await invoker<AgentSummary | null>('get_agent', { projectId, agentId });
    }

    throw new AgentBridgeUnavailableError();
  }

  async create(input: CreateAgentInput): Promise<CreateAgentOutput> {
    const invoker = getAgentInvoker();
    if (typeof invoker === 'function') {
      return await invoker<CreateAgentOutput>('create_agent', { input });
    }

    throw new AgentBridgeUnavailableError();
  }

  async update(input: UpdateAgentInput): Promise<UpdateAgentOutput> {
    const invoker = getAgentInvoker();
    if (typeof invoker === 'function') {
      return await invoker<UpdateAgentOutput>('update_agent', { input });
    }

    throw new AgentBridgeUnavailableError();
  }

  async archive(input: ArchiveAgentInput): Promise<ArchiveAgentOutput> {
    const invoker = getAgentInvoker();
    if (typeof invoker === 'function') {
      return await invoker<ArchiveAgentOutput>('archive_agent', { input });
    }

    throw new AgentBridgeUnavailableError();
  }
}

export const defaultAgentApi = new DesktopAgentApiClient();
