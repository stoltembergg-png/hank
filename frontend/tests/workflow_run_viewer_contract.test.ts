import { describe, expect, it } from 'vitest';
import { RunViewerModel, type RunSnapshot } from '../src/contracts/workflow-run-viewer';

const snapshot = (sequence: number, state: RunSnapshot['state'] = 'running'): RunSnapshot => ({
  project_id: 'project-a', run_id: 'run-a', generation: 2, sequence, state,
  nodes: [{ node_id: 'node-a', state, duration_ms: 12, outcome: null }],
  events: [
    { sequence, kind: 'transition', message: 'https://private.invalid/token=secret', timestamp_ms: sequence },
    { sequence: sequence + 1, kind: 'recovery', message: 'recovered', timestamp_ms: sequence + 1 },
  ],
});

describe('workflow run viewer contract', () => {
  // @spec:AC-1101
  it('projects bounded DAG state and deterministic timeline', () => {
    const viewer = new RunViewerModel('project-a', 2, 3);
    expect(viewer.apply(snapshot(1))).toBe(true);
    expect(viewer.snapshot?.nodes).toHaveLength(1);
    expect(viewer.timeline.map((event) => event.sequence)).toEqual([1, 2]);
    expect(viewer.snapshot?.state).toBe('running');
  });

  // @spec:AC-1102
  it('rejects stale generation/sequence and foreign project without overwrite', () => {
    const viewer = new RunViewerModel('project-a', 2, 3);
    expect(viewer.apply(snapshot(4))).toBe(true);
    expect(viewer.apply({ ...snapshot(3), generation: 2 })).toBe(false);
    expect(viewer.apply({ ...snapshot(5), project_id: 'project-b' })).toBe(false);
    expect(viewer.snapshot?.sequence).toBe(4);
  });

  // @spec:AC-1103
  it('redacts log content and keeps unknown/paused/recovered states explicit', () => {
    const viewer = new RunViewerModel('project-a', 2, 3);
    expect(viewer.apply(snapshot(1, 'unknown'))).toBe(true);
    expect(viewer.timeline[0].message).not.toContain('https://');
    expect(viewer.timeline[0].message).not.toContain('secret');
    expect(viewer.displayState).toBe('unknown');
    expect(viewer.canMutate).toBe(false);
  });
});
