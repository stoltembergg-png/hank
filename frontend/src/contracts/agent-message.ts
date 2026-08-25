export type AgentMessageStatus = 'pending' | 'result' | 'error' | 'terminated';
export type AgentMessageKind = 'data' | 'action-request';

export type AgentMessage = {
  message_id: string;
  project_id: string;
  group_id: string;
  session_id: string;
  trace_id: string;
  invocation_id: string;
  round: number;
  sender_id: string;
  receiver_id: string;
  kind: AgentMessageKind;
  status: AgentMessageStatus;
  text: string;
};

export type RenderedAgentMessage = {
  sender: string;
  receiver: string;
  trace: string;
  invocation: string;
  round: number;
  status: AgentMessageStatus;
  trust: 'untrusted-data';
  text: string;
  actionAllowed: false;
};

export class AgentMessageStore {
  public readonly messages: AgentMessage[] = [];

  public constructor(
    private readonly projectId: string,
    private readonly sessionId: string,
    private readonly identities = new Set<string>(),
  ) {}

  public apply(message: AgentMessage): boolean {
    if (
      message.project_id !== this.projectId ||
      message.session_id !== this.sessionId ||
      !message.message_id ||
      !message.trace_id ||
      !message.invocation_id ||
      message.round < 0 ||
      !Number.isInteger(message.round) ||
      (this.identities.size > 0 &&
        (!this.identities.has(message.sender_id) || !this.identities.has(message.receiver_id))) ||
      this.messages.some((item) => item.message_id === message.message_id)
    ) {
      return false;
    }
    this.messages.push({ ...message });
    return true;
  }
}

export function renderAgentMessage(message: AgentMessage): RenderedAgentMessage {
  return {
    sender: message.sender_id,
    receiver: message.receiver_id,
    trace: message.trace_id,
    invocation: message.invocation_id,
    round: message.round,
    status: message.status,
    trust: 'untrusted-data',
    text: escapeText(message.text),
    actionAllowed: false,
  };
}

function escapeText(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
