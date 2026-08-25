export const MAX_GROUP_CHAT_TEXT_BYTES = 65_536;

export type GroupChatStatus = 'active' | 'pending' | 'denied' | 'terminated';
export type GroupChatKind = 'message' | 'delegation' | 'policy' | 'session';

export type GroupChatEvent = {
  project_id: string;
  group_id: string;
  session_id: string;
  trace_id: string;
  sequence: number;
  agent_id: string;
  status: GroupChatStatus;
  kind: GroupChatKind;
  text: string;
};

export class GroupChatStore {
  public readonly messages: GroupChatEvent[] = [];
  public terminal = false;

  public constructor(
    private readonly projectId: string,
    private readonly sessionId: string,
    private readonly maxMessages: number,
  ) {}

  public apply(event: GroupChatEvent): boolean {
    if (!validEvent(event) || this.terminal) return false;
    if (event.project_id !== this.projectId || event.session_id !== this.sessionId) return false;
    const expected = this.messages.length;
    if (event.sequence !== expected || this.messages.some((item) => item.sequence === event.sequence)) {
      return false;
    }
    if (this.messages.length >= this.maxMessages) return false;
    this.messages.push({ ...event, text: event.text.slice(0, MAX_GROUP_CHAT_TEXT_BYTES) });
    this.terminal = event.status === 'terminated';
    return true;
  }
}

export function renderGroupChatText(text: string): string {
  return text
    .slice(0, MAX_GROUP_CHAT_TEXT_BYTES)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function validEvent(event: GroupChatEvent): boolean {
  return Boolean(
    event.project_id &&
      event.group_id &&
      event.session_id &&
      event.trace_id &&
      event.agent_id &&
      Number.isInteger(event.sequence) &&
      event.sequence >= 0 &&
      event.text.length <= MAX_GROUP_CHAT_TEXT_BYTES,
  );
}
