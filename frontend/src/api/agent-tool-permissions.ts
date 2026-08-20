export type PermissionEffect = 'allow' | 'ask' | 'deny';
export type PermissionScope = 'project' | 'agent' | 'session';
export type PermissionResource =
  | 'project'
  | 'agent'
  | 'session'
  | 'message'
  | 'memory'
  | 'skill'
  | 'tool'
  | 'workflow'
  | 'file'
  | 'process'
  | 'network'
  | 'secret'
  | 'provider'
  | 'plugin'
  | 'remote_node'
  | 'settings';
export type PermissionAction =
  | 'create'
  | 'read'
  | 'update'
  | 'delete'
  | 'list'
  | 'execute'
  | 'invoke'
  | 'delegate'
  | 'approve'
  | 'revoke'
  | 'configure'
  | 'discover'
  | 'stream'
  | 'cancel'
  | 'retry';

export interface PermissionCapability {
  resource: PermissionResource;
  action: PermissionAction;
  scope?: string | null;
}

export interface PermissionRule {
  capability: PermissionCapability;
  effect: PermissionEffect;
  scope: PermissionScope;
  scope_id: string;
  expires_at?: string | null;
}

export interface ToolPermissionPolicy {
  schema_version: number;
  default_effect: PermissionEffect;
  rules: PermissionRule[];
}

export interface ToolPermissionPolicySnapshot {
  policy: ToolPermissionPolicy;
  updated_at: string;
}

export interface UpdateToolPermissionPolicyInput {
  project_id: string;
  agent_id: string;
  policy: ToolPermissionPolicy;
  expected_version: string;
}

export interface ToolPermissionApiClient {
  get(projectId: string, agentId: string): Promise<ToolPermissionPolicySnapshot | null>;
  update(input: UpdateToolPermissionPolicyInput): Promise<ToolPermissionPolicySnapshot>;
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

export class DesktopToolPermissionApiClient implements ToolPermissionApiClient {
  async get(projectId: string, agentId: string): Promise<ToolPermissionPolicySnapshot | null> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<ToolPermissionPolicySnapshot | null>('get_agent_tool_permissions', {
        projectId,
        agentId,
      });
    }

    // Sem Permission Engine, o default deny permanece implícito e não há grant inventado.
    return null;
  }

  async update(input: UpdateToolPermissionPolicyInput): Promise<ToolPermissionPolicySnapshot> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<ToolPermissionPolicySnapshot>('update_agent_tool_permissions', { input });
    }

    throw new Error('No Permission Engine available');
  }
}

export const defaultToolPermissionApi = new DesktopToolPermissionApiClient();