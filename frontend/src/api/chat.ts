import {
  CHAT_STREAM_EVENT_NAME,
  type ChatStreamEvent,
} from '@/contracts/chat-stream';
import type {
  ChatCommandRequest,
  ChatHistoryMessage,
  ChatSessionScope,
  ChatTransport,
} from '@/chat/ChatPage';

type BridgeInvoker = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
type UnlistenFn = () => void;

interface BridgeWindow {
  __TAURI_INTERNALS__?: {
    invoke?: BridgeInvoker;
    transformCallback?: (callback: (event: unknown) => void, once?: boolean) => number;
  };
  __TAURI_INVOKE__?: BridgeInvoker;
  __TAURI_EVENT_PLUGIN_INTERNALS__?: {
    unregisterListener?: (event: string, eventId: number) => void;
  };
}

function bridgeInvoker(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridge = window as unknown as BridgeWindow;
  return bridge.__TAURI_INTERNALS__?.invoke ?? bridge.__TAURI_INVOKE__;
}

function listenToChatStream(listener: (event: ChatStreamEvent) => void): Promise<UnlistenFn> {
  if (typeof window === 'undefined') return Promise.resolve(() => undefined);
  const bridge = window as unknown as BridgeWindow;
  const internals = bridge.__TAURI_INTERNALS__;
  if (typeof internals?.invoke !== 'function' || typeof internals.transformCallback !== 'function') {
    return Promise.reject(new ChatBridgeUnavailableError());
  }
  const callbackId = internals.transformCallback((value) => {
    if (isEventEnvelope(value)) listener(value.payload);
  });
  return internals.invoke<number>('plugin:event|listen', {
    event: CHAT_STREAM_EVENT_NAME,
    target: { kind: 'Any' },
    handler: callbackId,
  }).then((eventId) => () => {
    bridge.__TAURI_EVENT_PLUGIN_INTERNALS__?.unregisterListener?.(CHAT_STREAM_EVENT_NAME, eventId);
    void internals.invoke?.('plugin:event|unlisten', {
      event: CHAT_STREAM_EVENT_NAME,
      eventId,
    });
  });
}

function isEventEnvelope(value: unknown): value is { payload: ChatStreamEvent } {
  return typeof value === 'object' && value !== null && 'payload' in value;
}

export const CHAT_BRIDGE_UNAVAILABLE_CODE = 'CHAT_BRIDGE_UNAVAILABLE' as const;

export class ChatBridgeUnavailableError extends Error {
  readonly code = CHAT_BRIDGE_UNAVAILABLE_CODE;

  constructor() {
    super('Chat desktop bridge is unavailable');
    this.name = 'ChatBridgeUnavailableError';
  }
}

/** Tauri transport for the typed chat command and stream event contracts. */
export class DesktopChatTransport implements ChatTransport {
  private readonly listeners = new Set<(event: unknown) => void>();
  private eventReady: Promise<UnlistenFn> | null = null;

  subscribe(listener: (event: unknown) => void): () => void {
    if (!bridgeInvoker()) return () => undefined;
    this.listeners.add(listener);
    this.eventReady ??= listenToChatStream((event) => {
      for (const current of this.listeners) current(event);
    });
    return () => {
      this.listeners.delete(listener);
    };
  }

  async send(request: ChatCommandRequest): Promise<void> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new ChatBridgeUnavailableError();
    await this.eventReady;
    await invoke('send_chat_command', { command: request });
  }

  async cancel(input: { command_id: string; session_id: string; caller: ChatCommandRequest['caller'] }): Promise<void> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new ChatBridgeUnavailableError();
    await invoke('cancel_chat_command', { input });
  }

  async loadMessages(session: ChatSessionScope): Promise<ChatHistoryMessage[]> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new ChatBridgeUnavailableError();
    const result = await invoke<{
      messages: Array<{
        id: string;
        role: string;
        text: string;
      }>;
      limit: number;
      offset: number;
    }>('list_chat_messages', {
      input: {
        project_id: session.project_id,
        agent_id: session.agent_id,
        session_id: session.session_id,
        caller: session.caller,
        limit: 100,
        offset: 0,
      },
    });
    return result.messages
      .filter((message): message is ChatHistoryMessage =>
        (message.role === 'user' || message.role === 'assistant') && typeof message.text === 'string')
      .map((message) => ({ id: message.id, role: message.role, text: message.text }));
  }
}

export const defaultChatTransport = new DesktopChatTransport();
