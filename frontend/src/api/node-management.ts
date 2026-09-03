export type NodeState = 'active' | 'expired' | 'revoked' | 'unknown';
export type NodeHealth = 'healthy' | 'stale' | 'unreachable' | 'unknown';

export interface NodeStatus {
  node_id: string;
  peer_id: string;
  project_id: string;
  display_name: string;
  state: NodeState;
  health: NodeHealth;
  capabilities: string[];
  authenticated_at_ms: number;
  last_seen_ms: number;
  stale_since_ms: number | null;
  actor: string;
  protocol_revision: string;
}

export interface NodeListResult {
  nodes: NodeStatus[];
  fetched_at_ms: number;
}

export interface NodeRevokeInput {
  project_id: string;
  node_id: string;
  actor: string;
}

export interface NodeRevokeResult {
  node_id: string;
  state: 'revoked';
  revoked_at_ms: number;
}

export interface NodeManagementApiClient {
  list(projectId: string): Promise<NodeListResult>;
  get(projectId: string, nodeId: string): Promise<NodeStatus>;
  revoke(input: NodeRevokeInput): Promise<NodeRevokeResult>;
}

type BridgeInvoker = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface BridgeWindow {
  __TAURI_INTERNALS__?: { invoke?: BridgeInvoker };
  __TAURI_INVOKE__?: BridgeInvoker;
}

export const STALE_RESPONSE_THRESHOLD_MS = 30_000;

function bridgeInvoker(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridge = window as unknown as BridgeWindow;
  return bridge.__TAURI_INTERNALS__?.invoke ?? bridge.__TAURI_INVOKE__;
}

export class DesktopNodeManagementApiClient implements NodeManagementApiClient {
  async list(projectId: string): Promise<NodeListResult> {
    const invoke = bridgeInvoker();
    if (!invoke) {
      return { nodes: [], fetched_at_ms: 0 };
    }
    return invoke<NodeListResult>('list_nodes', { projectId });
  }

  async get(projectId: string, nodeId: string): Promise<NodeStatus> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new Error('Node management service unavailable');
    return invoke<NodeStatus>('get_node', { projectId, nodeId });
  }

  async revoke(input: NodeRevokeInput): Promise<NodeRevokeResult> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new Error('Node management service unavailable');
    return invoke<NodeRevokeResult>('revoke_node', { input });
  }
}

export const defaultNodeManagementApi = new DesktopNodeManagementApiClient();

export function isStaleResponse(result: NodeListResult, nowMs: number): boolean {
  if (result.fetched_at_ms <= 0) return true;
  return nowMs - result.fetched_at_ms > STALE_RESPONSE_THRESHOLD_MS;
}
