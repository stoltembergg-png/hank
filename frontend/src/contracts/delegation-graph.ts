export const MAX_DELEGATION_LABEL_BYTES = 512;

export type DelegationStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled' | 'denied';
export type DelegationBudgetState = 'available' | 'reserved' | 'exceeded';
export type DelegationDenialReason = 'cycle' | 'depth' | 'budget' | 'scope' | 'policy';

export type DelegationGraphEvent = {
  event_id: string;
  project_id: string;
  session_id: string;
  trace_id: string;
  invocation_id: string;
  parent_id: string | null;
  status: DelegationStatus;
  depth: number;
  round: number;
  budget_state: DelegationBudgetState;
  denial_reason: DelegationDenialReason | null;
  label: string;
};

export type DelegationGraphNode = DelegationGraphEvent & { x: number };
export type DelegationGraphEdge = { parent_id: string; child_id: string };

export class DelegationGraphStore {
  public readonly nodes: DelegationGraphNode[] = [];
  public readonly edges: DelegationGraphEdge[] = [];

  public constructor(
    private readonly projectId: string,
    private readonly sessionId: string,
    private readonly maxNodes: number,
  ) {}

  public apply(event: DelegationGraphEvent): boolean {
    if (!validEvent(event) || event.project_id !== this.projectId || event.session_id !== this.sessionId) {
      return false;
    }
    if (this.nodes.length >= this.maxNodes || this.nodes.some((node) => node.invocation_id === event.invocation_id)) {
      return false;
    }
    if (event.parent_id !== null && !this.nodes.some((node) => node.invocation_id === event.parent_id)) {
      return false;
    }
    const node: DelegationGraphNode = { ...event, label: event.label.slice(0, MAX_DELEGATION_LABEL_BYTES), x: this.nodes.length };
    this.nodes.push(node);
    if (node.parent_id !== null) this.edges.push({ parent_id: node.parent_id, child_id: node.invocation_id });
    return true;
  }

  public cancel(_invocationId: string): false {
    return false;
  }
}

export function renderDelegationLabel(label: string): string {
  return label
    .slice(0, MAX_DELEGATION_LABEL_BYTES)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function validEvent(event: DelegationGraphEvent): boolean {
  return Boolean(
    event.event_id &&
      event.project_id &&
      event.session_id &&
      event.trace_id &&
      event.invocation_id &&
      Number.isInteger(event.depth) &&
      event.depth >= 0 &&
      Number.isInteger(event.round) &&
      event.round >= 0 &&
      event.label.length <= MAX_DELEGATION_LABEL_BYTES,
  );
}
