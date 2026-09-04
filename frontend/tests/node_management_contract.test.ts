import { describe, expect, it } from 'vitest';
import { isStaleResponse, STALE_RESPONSE_THRESHOLD_MS, NodeListResult } from '@/api/node-management';

function makeResult(fetchedAtMs: number, nodes = 1): NodeListResult {
  return {
    fetched_at_ms: fetchedAtMs,
    nodes: Array.from({ length: nodes }, (_, i) => ({
      node_id: `node-${i}`,
      peer_id: `peer-${i}`,
      project_id: 'proj-1',
      display_name: `Node ${i}`,
      state: 'active' as const,
      health: 'healthy' as const,
      capabilities: ['observe'],
      authenticated_at_ms: fetchedAtMs,
      last_seen_ms: fetchedAtMs,
      stale_since_ms: null,
      actor: 'agent-1',
      protocol_revision: '1.0',
    })),
  };
}

describe('node-management staleness contract', () => {
  it('AC-1474: marks responses older than the threshold as stale', () => {
    const now = 1_000_000;
    const result = makeResult(now - STALE_RESPONSE_THRESHOLD_MS - 1);
    expect(isStaleResponse(result, now)).toBe(true);
  });

  it('AC-1474: keeps fresh responses within the threshold as actionable', () => {
    const now = 1_000_000;
    const result = makeResult(now - 5_000);
    expect(isStaleResponse(result, now)).toBe(false);
  });

  it('AC-1474: treats zero fetched_at_ms as stale (no data available)', () => {
    const result = makeResult(0);
    expect(isStaleResponse(result, 1_000_000)).toBe(true);
  });
});
