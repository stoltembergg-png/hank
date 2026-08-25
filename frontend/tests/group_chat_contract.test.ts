import { describe, expect, it } from 'vitest';
import {
  GroupChatEvent,
  GroupChatStore,
  MAX_GROUP_CHAT_TEXT_BYTES,
  renderGroupChatText,
} from '../src/contracts/group-chat';

const base = { project_id: 'proj-a', group_id: 'group-a', session_id: 'session-a', trace_id: 'trace-a' };

function event(overrides: Partial<GroupChatEvent>): GroupChatEvent {
  return { ...base, sequence: 0, kind: 'message', agent_id: 'agent-a', status: 'active', text: 'hello', ...overrides };
}

describe('group chat event contract', () => {
  // @spec:AC-930
  it('accepts only current project/session events and preserves provenance', () => {
    const store = new GroupChatStore(base.project_id, base.session_id, 10);
    expect(store.apply(event({}))).toBe(true);
    expect(store.messages[0]).toMatchObject({ agent_id: 'agent-a', trace_id: 'trace-a' });
    expect(store.apply(event({ sequence: 0 }))).toBe(false);
    expect(store.apply(event({ sequence: 1, project_id: 'proj-b' }))).toBe(false);
  });

  // @spec:AC-931
  it('renders terminal, pending and denied states and truncates content', () => {
    const store = new GroupChatStore(base.project_id, base.session_id, 10);
    store.apply(event({ status: 'pending', kind: 'delegation' }));
    store.apply(event({ sequence: 1, status: 'denied', kind: 'policy', text: '<script>alert(1)</script>' }));
    store.apply(event({ sequence: 2, status: 'terminated', kind: 'session' }));
    expect(store.terminal).toBe(true);
    const rendered = renderGroupChatText(store.messages[1].text);
    expect(rendered).not.toContain('<script>');
    expect(rendered).toContain('&lt;script&gt;');
  });

  // @spec:AC-932
  it('bounds large output and rejects invalid sequence or oversized text', () => {
    const store = new GroupChatStore(base.project_id, base.session_id, 2);
    expect(store.apply(event({ text: 'x'.repeat(MAX_GROUP_CHAT_TEXT_BYTES + 1) }))).toBe(false);
    expect(store.apply(event({}))).toBe(true);
    expect(store.apply(event({ sequence: 2 }))).toBe(false);
    expect(store.messages).toHaveLength(1);
  });
});
