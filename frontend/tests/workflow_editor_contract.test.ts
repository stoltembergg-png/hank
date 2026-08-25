import { describe, expect, it, vi } from 'vitest';
import { WorkflowEditorModel, type WorkflowCommand, type WorkflowNode } from '../src/contracts/workflow-editor';

const node = (id: string, kind = 'agent'): WorkflowNode => ({ id, kind, label: id });

describe('workflow editor contract', () => {
  // @spec:AC-1081
  it('keeps a bounded valid DAG and rejects invalid edges without mutation', () => {
    const model = new WorkflowEditorModel('project-a', 'workflow-a', 3, 3, 7);
    expect(model.addNode(node('a'))).toBe(true);
    expect(model.addNode(node('b'))).toBe(true);
    expect(model.addEdge('a', 'b')).toBe(true);
    expect(model.addEdge('b', 'a')).toBe(false);
    expect(model.addEdge('missing', 'b')).toBe(false);
    expect(model.addEdge('a', 'a')).toBe(false);
    expect(model.edges).toHaveLength(1);
  });

  // @spec:AC-1082
  it('preserves scope/version and escapes untrusted labels in command output', () => {
    const model = new WorkflowEditorModel('project-a', 'workflow-a', 3, 3, 64);
    expect(model.addNode({ id: 'a', kind: 'agent', label: '<script>inject</script>' })).toBe(true);
    const command = model.command(4);
    expect(command.project_id).toBe('project-a');
    expect(command.expected_version).toBe(4);
    expect(command.draft.nodes[0].label).toContain('&lt;script&gt;');
    expect(command.draft.nodes[0].label).not.toContain('<script>');
  });

  // @spec:AC-1083
  it('does not mutate on duplicate submit or stale validation', async () => {
    const model = new WorkflowEditorModel('project-a', 'workflow-a', 3, 3, 7);
    model.addNode(node('a'));
    const api = {
      validate: vi.fn(async (_command: WorkflowCommand) => ({ valid: false, reason: 'stale_version' })),
      save: vi.fn(async (_command: WorkflowCommand) => ({ version: 8 })),
    };
    await expect(model.submit(api, 7)).rejects.toThrow('stale_version');
    await expect(model.submit(api, 7)).rejects.toThrow('duplicate_submit');
    expect(api.save).not.toHaveBeenCalled();
  });
});
