export type RunState = 'pending' | 'running' | 'paused' | 'unknown' | 'recovered' | 'completed' | 'failed';
export type RunNode = { node_id: string; state: RunState; duration_ms: number; outcome: string | null };
export type RunEvent = { sequence: number; kind: 'start' | 'transition' | 'end' | 'error' | 'recovery'; message: string; timestamp_ms: number };
export type RunSnapshot = { project_id: string; run_id: string; generation: number; sequence: number; state: RunState; nodes: RunNode[]; events: RunEvent[] };

export class RunViewerModel {
  public snapshot: RunSnapshot | null = null;
  public timeline: RunEvent[] = [];
  public readonly canMutate = false;
  public constructor(private readonly projectId: string, private readonly maxNodes: number, private readonly maxEvents: number) {
    if (!projectId || maxNodes < 1 || maxNodes > 256 || maxEvents < 1 || maxEvents > 1024) throw new Error('invalid_viewer_bounds');
  }
  public get displayState(): RunState | null { return this.snapshot?.state ?? null; }
  public apply(next: RunSnapshot): boolean {
    if (next.project_id !== this.projectId || !validId(next.run_id) || !Number.isInteger(next.generation) || !Number.isInteger(next.sequence) || next.nodes.length > this.maxNodes || next.events.length > this.maxEvents || next.nodes.some((node) => !validId(node.node_id)) || next.events.some((event) => event.sequence < 0)) return false;
    if (this.snapshot && (next.run_id !== this.snapshot.run_id || next.generation < this.snapshot.generation || (next.generation === this.snapshot.generation && next.sequence < this.snapshot.sequence))) return false;
    const events = next.events.map((event) => ({ ...event, message: redact(event.message) })).sort((left, right) => left.sequence - right.sequence || left.timestamp_ms - right.timestamp_ms);
    if (events.some((event, index) => index > 0 && event.sequence === events[index - 1].sequence)) return false;
    this.snapshot = { ...next, nodes: next.nodes.map((node) => ({ ...node })), events };
    this.timeline = events;
    return true;
  }
}
function validId(value: string): boolean { return value.length > 0 && value.length <= 128 && [...value].every((character) => character.charCodeAt(0) > 31 && character.charCodeAt(0) !== 127); }
function redact(value: string): string { return value.replace(/https?:\/\/\S+/gi, '[redacted-url]').replace(/(?:token|secret|password)\s*[:=]\s*\S+/gi, '[redacted-secret]').replace(/(?:\/home\/|\.\.\/|[A-Za-z]:\\)\S*/g, '[redacted-path]').replace(/page content/gi, '[redacted-content]'); }
