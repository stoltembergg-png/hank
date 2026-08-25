export const MAX_WORKFLOW_NODES = 128;
export const MAX_WORKFLOW_EDGES = 256;
export const MAX_WORKFLOW_LABEL_BYTES = 256;

export type WorkflowNode = { id: string; kind: string; label: string };
export type WorkflowEdge = { source: string; target: string };
export type WorkflowDraft = { project_id: string; workflow_id: string; nodes: WorkflowNode[]; edges: WorkflowEdge[] };
export type WorkflowCommand = { project_id: string; workflow_id: string; expected_version: number; draft: WorkflowDraft };
export type WorkflowValidation = { valid: boolean; reason?: string };
export type WorkflowApi = { validate(command: WorkflowCommand): Promise<WorkflowValidation>; save(command: WorkflowCommand): Promise<{ version: number }> };

export class WorkflowEditorModel {
  public readonly nodes: WorkflowNode[] = [];
  public readonly edges: WorkflowEdge[] = [];
  private readonly submitted = new Set<string>();
  public constructor(private readonly projectId: string, private readonly workflowId: string, private readonly maxNodes: number, private readonly maxEdges: number, private readonly maxLabelBytes: number) {
    if (!validId(projectId) || !validId(workflowId) || maxNodes < 1 || maxNodes > MAX_WORKFLOW_NODES || maxEdges < 1 || maxEdges > MAX_WORKFLOW_EDGES || maxLabelBytes < 1 || maxLabelBytes > MAX_WORKFLOW_LABEL_BYTES) throw new Error('invalid_editor_bounds');
  }
  public addNode(node: WorkflowNode): boolean {
    if (!validId(node.id) || !validId(node.kind) || node.label.length > this.maxLabelBytes || this.nodes.length >= this.maxNodes || this.nodes.some((item) => item.id === node.id)) return false;
    this.nodes.push({ ...node });
    return true;
  }
  public addEdge(source: string, target: string): boolean {
    if (!validId(source) || !validId(target) || source === target || this.edges.length >= this.maxEdges || !this.nodes.some((node) => node.id === source) || !this.nodes.some((node) => node.id === target) || this.edges.some((edge) => edge.source === source && edge.target === target)) return false;
    const candidate = [...this.edges, { source, target }];
    if (hasCycle(this.nodes.map((node) => node.id), candidate)) return false;
    this.edges.push({ source, target });
    return true;
  }
  public command(expectedVersion: number): WorkflowCommand {
    if (!Number.isInteger(expectedVersion) || expectedVersion < 0) throw new Error('invalid_expected_version');
    return { project_id: this.projectId, workflow_id: this.workflowId, expected_version: expectedVersion, draft: { project_id: this.projectId, workflow_id: this.workflowId, nodes: this.nodes.map((node) => ({ ...node, label: escapeLabel(node.label) })), edges: this.edges.map((edge) => ({ ...edge })) } };
  }
  public async submit(api: WorkflowApi, expectedVersion: number): Promise<number> {
    const command = this.command(expectedVersion);
    const key = `${command.project_id}:${command.workflow_id}:${command.expected_version}:${JSON.stringify(command.draft)}`;
    if (this.submitted.has(key)) throw new Error('duplicate_submit');
    this.submitted.add(key);
    const validation = await api.validate(command);
    if (!validation.valid) throw new Error(validation.reason ?? 'workflow_validation_failed');
    const result = await api.save(command);
    return result.version;
  }
}

export function escapeLabel(value: string): string { return value.slice(0, MAX_WORKFLOW_LABEL_BYTES).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;'); }
function validId(value: string): boolean {
  return value.length > 0 && value.length <= 128 && [...value].every((character) => {
    const code = character.charCodeAt(0);
    return code > 31 && code !== 127;
  });
}
function hasCycle(nodes: string[], edges: WorkflowEdge[]): boolean {
  const graph = new Map(nodes.map((node) => [node, [] as string[]]));
  edges.forEach((edge) => graph.get(edge.source)?.push(edge.target));
  const visiting = new Set<string>(); const visited = new Set<string>();
  const visit = (node: string): boolean => { if (visiting.has(node)) return true; if (visited.has(node)) return false; visiting.add(node); if ((graph.get(node) ?? []).some(visit)) return true; visiting.delete(node); visited.add(node); return false; };
  return nodes.some(visit);
}
