export type AutonomyLevel =
  | 'l0_none'
  | 'l1_assisted'
  | 'l2_semi_autonomous'
  | 'l3_autonomous'
  | 'l4_fully_autonomous';

export type AutonomyOperation =
  | 'read_data'
  | 'execute_safe_tool'
  | 'execute_stateful_tool'
  | 'spawn_sub_agent'
  | 'create_workflow'
  | 'modify_skill'
  | 'access_external_network'
  | 'modify_system_config';

export type AutonomyDecision = 'allow' | 'require_human_approval' | 'deny';

export interface AutonomyPolicy {
  schema_version: number;
  level: AutonomyLevel;
  allow_subagents: boolean;
  allow_workflow_creation: boolean;
  allow_skill_modification: boolean;
  allow_network_access: boolean;
  max_consecutive_autonomous_steps: number;
}

export interface AutonomyTransitionApproval {
  approver_id: string;
  reason: string;
  expires_at?: string | null;
}

export interface AutonomyPolicySnapshot {
  policy: AutonomyPolicy;
  decisions: Record<AutonomyOperation, AutonomyDecision>;
  updated_at: string;
}

export interface UpdateAutonomyInput {
  project_id: string;
  agent_id: string;
  policy: AutonomyPolicy;
  approval?: AutonomyTransitionApproval;
  expected_version: string;
}

export interface AutonomyApiClient {
  get(projectId: string, agentId: string): Promise<AutonomyPolicySnapshot | null>;
  update(input: UpdateAutonomyInput): Promise<AutonomyPolicySnapshot>;
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

export class DesktopAutonomyApiClient implements AutonomyApiClient {
  async get(projectId: string, agentId: string): Promise<AutonomyPolicySnapshot | null> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<AutonomyPolicySnapshot | null>('get_agent_autonomy_policy', {
        projectId,
        agentId,
      });
    }
    return null;
  }

  async update(input: UpdateAutonomyInput): Promise<AutonomyPolicySnapshot> {
    const invoker = bridgeInvoker();
    if (typeof invoker === 'function') {
      return await invoker<AutonomyPolicySnapshot>('update_agent_autonomy_policy', { input });
    }
    throw new Error('No autonomy service available');
  }
}

export const defaultAutonomyApi = new DesktopAutonomyApiClient();