import { describe, expect, it } from 'vitest';
import { DelegationGraphStore, renderDelegationLabel, type DelegationGraphEvent } from '../src/contracts/delegation-graph';

const base: DelegationGraphEvent = {
  event_id: 'event-a', project_id: 'proj-a', session_id: 'session-a', trace_id: 'trace-a',
  invocation_id: 'invoke-a', parent_id: null, status: 'pending', depth: 0, round: 1,
  budget_state: 'available', denial_reason: null, label: 'root',
};

describe('delegation graph contract', () => {
  // @spec:AC-940
  it('renders acyclic parent-child nodes with stable deterministic layout', () => {
    const store = new DelegationGraphStore('proj-a', 'session-a', 10);
    expect(store.apply(base)).toBe(true);
    expect(store.apply({ ...base, event_id: 'event-b', invocation_id: 'invoke-b', parent_id: 'invoke-a', depth: 1, label: 'child' })).toBe(true);
    expect(store.nodes.map((node) => node.x)).toEqual([0, 1]);
    expect(store.edges).toEqual([{ parent_id: 'invoke-a', child_id: 'invoke-b' }]);
  });

  // @spec:AC-941
  it('shows cycle depth and budget denial reasons without allowing mutation', () => {
    const store = new DelegationGraphStore('proj-a', 'session-a', 10);
    store.apply({ ...base, status: 'denied', denial_reason: 'cycle' });
    expect(store.nodes[0].denial_reason).toBe('cycle');
    expect(store.cancel('invoke-a')).toBe(false);
  });

  // @spec:AC-942
  it('dedupes events, rejects foreign scope and truncates labels', () => {
    const store = new DelegationGraphStore('proj-a', 'session-a', 1);
    expect(store.apply(base)).toBe(true);
    expect(store.apply(base)).toBe(false);
    expect(store.apply({ ...base, event_id: 'event-b', invocation_id: 'invoke-b', project_id: 'proj-b' })).toBe(false);
    expect(store.apply({ ...base, event_id: 'event-c', invocation_id: 'invoke-c' })).toBe(false);
    expect(renderDelegationLabel('<script>inject</script>')).toContain('&lt;script&gt;');
  });
});
