export type InstructionLayer = 'agent';
export type InstructionProvenance = 'agent' | 'project' | 'user';

export interface AgentInstructionSnapshot {
  layer: InstructionLayer;
  content: string;
  max_total_bytes: number;
  provenance: InstructionProvenance;
  updated_at: string;
}

export interface UpdateAgentInstructionInput {
  project_id: string;
  agent_id: string;
  layer: 'agent';
  content: string;
  max_total_bytes: number;
  expected_version: string;
}

export interface AgentInstructionApiClient {
  get(projectId: string, agentId: string): Promise<AgentInstructionSnapshot | null>;
  update(input: UpdateAgentInstructionInput): Promise<AgentInstructionSnapshot>;
}

type BridgeInvoker = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: { invoke?: BridgeInvoker };
  __TAURI_INVOKE__?: BridgeInvoker;
}

function bridgeInvoker(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

export class DesktopAgentInstructionApiClient implements AgentInstructionApiClient {
  async get(projectId: string, agentId: string): Promise<AgentInstructionSnapshot | null> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<AgentInstructionSnapshot | null>('get_agent_instruction', {
        projectId,
        agentId,
      });
    }

    return null;
  }

  async update(input: UpdateAgentInstructionInput): Promise<AgentInstructionSnapshot> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<AgentInstructionSnapshot>('update_agent_instruction', { input });
    }

    throw new Error('No instruction service available');
  }
}

export const defaultAgentInstructionApi = new DesktopAgentInstructionApiClient();