type BridgeInvoker = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

export type ModelModality = 'text' | 'image' | 'audio' | 'video';
export type CapabilityState = 'supported' | 'unsupported' | 'unknown';
export type ProviderState = 'available' | 'unsupported' | 'unknown';

export interface ModelPolicy {
  provider: string;
  model: string;
  max_tokens?: number;
  max_context_tokens?: number;
  temperature?: number;
  modalities: ModelModality[];
}

export type ModelCapabilities = Record<ModelModality, CapabilityState>;

export interface ModelPolicySnapshot {
  policy: ModelPolicy;
  capabilities: ModelCapabilities;
  provider_state: ProviderState;
  updated_at: string;
}

export interface UpdateModelPolicyInput {
  project_id: string;
  agent_id: string;
  policy: ModelPolicy;
  expected_version: string;
}

export interface ModelPolicyApiClient {
  get(projectId: string, agentId: string): Promise<ModelPolicySnapshot | null>;
  update(input: UpdateModelPolicyInput): Promise<ModelPolicySnapshot>;
}

interface InjectedBridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
  };
  __TAURI_INVOKE__?: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

function bridgeInvoker(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridgeWin = window as unknown as InjectedBridgeWindow;
  return bridgeWin.__TAURI_INTERNALS__?.invoke ?? bridgeWin.__TAURI_INVOKE__;
}

export class DesktopModelPolicyApiClient implements ModelPolicyApiClient {
  async get(projectId: string, agentId: string): Promise<ModelPolicySnapshot | null> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<ModelPolicySnapshot | null>('get_agent_model_policy', {
        projectId,
        agentId,
      });
    }

    // Sem bridge/provider, a UI deve mostrar unsupported/no-provider explicitamente.
    return null;
  }

  async update(input: UpdateModelPolicyInput): Promise<ModelPolicySnapshot> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<ModelPolicySnapshot>('update_agent_model_policy', { input });
    }

    throw new Error('No model policy provider available');
  }
}

export const defaultModelPolicyApi = new DesktopModelPolicyApiClient();
