import { FormEvent, Dispatch, SetStateAction, useEffect, useRef, useState } from 'react';
import {
  ChatStreamConsumer,
  type ChatStreamSubscription,
} from '@/contracts/chat-stream';
import { SafeMarkdown } from './markdown/SafeMarkdown';
import { ProviderIndicator, type ProviderIndicatorData } from './indicators/ProviderIndicator';
import { ToolCallCard, type ToolCallViewModel } from './tool-call/ToolCallCard';
import { UsageSummary, type UsageReadModel } from './usage/UsageSummary';
import './ChatPage.css';

export type ChatSessionScope = Omit<ChatStreamSubscription, 'stream_id' | 'command_id'>;

export type ChatCommandRequest = ChatSessionScope & {
  schema_version: 1;
  command_id: string;
  stream_id: string;
  cancellation_id: string;
  text: string;
};

export type ChatHistoryMessage = {
  id: string;
  role: 'user' | 'assistant';
  text: string;
};

export type ChatSendResult = {
  command_id: string;
  stream_id: string;
  state: 'completed' | 'cancelled';
  provider_id: string;
  model_id: string;
  provider_state: ProviderIndicatorData['state'];
  capability: ProviderIndicatorData['capability'];
  attempt_number: number;
};

export type ChatTransport = {
  send: (request: ChatCommandRequest) => Promise<ChatSendResult | void>;
  cancel: (input: {
    command_id: string;
    session_id: string;
    caller: ChatSessionScope['caller'];
  }) => Promise<void>;
  loadMessages?: (session: ChatSessionScope) => Promise<ChatHistoryMessage[]>;
  loadUsage?: (session: ChatSessionScope) => Promise<UsageReadModel | null>;
  subscribe: (listener: (event: unknown) => void) => () => void;
};

export type ChatIdFactory = () => { command_id: string; stream_id: string };

type ChatStatus = 'idle' | 'sending' | 'streaming' | 'cancelling' | 'completed' | 'cancelled' | 'error';
type RenderedMessage = { id: string; role: 'user' | 'assistant'; text: string };
type ActiveTurn = {
  request: ChatCommandRequest;
  consumer: ChatStreamConsumer;
  assistantId: string;
};

const MAX_MESSAGES = 200;

export function ChatPage({
  session,
  transport,
  createIds = defaultIds,
  indicator,
  usage,
  toolCalls = [],
  onApproveToolCall,
  initialMessages = [],
}: {
  session: ChatSessionScope;
  transport: ChatTransport;
  createIds?: ChatIdFactory;
  indicator?: ProviderIndicatorData;
  usage?: UsageReadModel;
  toolCalls?: ToolCallViewModel[];
  onApproveToolCall?: (call: ToolCallViewModel) => void;
  initialMessages?: ChatHistoryMessage[];
}) {
  const [draft, setDraft] = useState('');
  const [messages, setMessages] = useState<RenderedMessage[]>(initialMessages);
  const [status, setStatus] = useState<ChatStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [lastText, setLastText] = useState<string | null>(null);
  const [resolvedIndicator, setResolvedIndicator] = useState(indicator);
  const [resolvedUsage, setResolvedUsage] = useState(usage);
  const activeTurn = useRef<ActiveTurn | null>(null);
  const latestCommandId = useRef<string | null>(null);

  useEffect(() => {
    setResolvedIndicator(indicator);
  }, [indicator]);

  useEffect(() => {
    setResolvedUsage(usage);
  }, [usage]);

  useEffect(() => {
    return transport.subscribe((value) => {
      const turn = activeTurn.current;
      if (!turn) return;
      const result = turn.consumer.accept(value);
      if (!result.accepted) return;

      const { event } = result;
      switch (event.payload.kind) {
        case 'start':
          setStatus('streaming');
          ensureAssistant(setMessages, turn.assistantId);
          break;
        case 'delta':
          setStatus('streaming');
          appendAssistant(setMessages, turn.assistantId, event.payload.text);
          break;
        case 'finish':
          setStatus('completed');
          activeTurn.current = null;
          break;
        case 'cancel':
          setStatus('cancelled');
          activeTurn.current = null;
          break;
        case 'error':
          setError('A geração não pôde ser concluída. Tente novamente.');
          setStatus('error');
          activeTurn.current = null;
          break;
        case 'usage':
          break;
      }
    });
  }, [transport]);

  useEffect(() => {
    if (!transport.loadMessages) return undefined;
    let disposed = false;
    void transport.loadMessages(session)
      .then((loaded) => {
        if (!disposed && !activeTurn.current) setMessages(loaded);
      })
      .catch(() => {
        if (!disposed) setError('Não foi possível carregar o histórico da sessão.');
      });
    return () => {
      disposed = true;
    };
  }, [session, transport]);

  useEffect(() => {
    if (!transport.loadUsage) return undefined;
    let disposed = false;
    void transport.loadUsage(session)
      .then((loaded) => {
        if (!disposed && loaded) setResolvedUsage(loaded);
      })
      .catch(() => {
        if (!disposed) setError('Não foi possível carregar as métricas da sessão.');
      });
    return () => {
      disposed = true;
    };
  }, [session, transport]);

  const busy = status === 'sending' || status === 'streaming' || status === 'cancelling';

  async function beginTurn(text: string, appendUser: boolean) {
    const ids = createIds();
    const request: ChatCommandRequest = {
      schema_version: 1,
      ...session,
      ...ids,
      cancellation_id: `cancel-${ids.command_id}`,
      text,
    };
    activeTurn.current = {
      request,
      consumer: new ChatStreamConsumer({ ...session, ...ids }),
      assistantId: `assistant-${ids.command_id}`,
    };
    latestCommandId.current = request.command_id;
    setLastText(text);
    setError(null);
    setStatus('sending');
    if (appendUser) {
      setMessages((current) => trimMessages([
        ...current,
        { id: `user-${ids.command_id}`, role: 'user', text },
      ]));
    }
    setDraft('');
    try {
      const result = await transport.send(request);
      if (isChatSendResult(result)) {
        setResolvedIndicator({
          provider_id: result.provider_id,
          model_id: result.model_id,
          state: result.provider_state,
          capability: result.capability,
          attempt_number: result.attempt_number,
        });
      }
      if (transport.loadUsage) {
        try {
          const currentUsage = await transport.loadUsage(session);
          if (latestCommandId.current === request.command_id) setResolvedUsage(currentUsage ?? undefined);
        } catch {
          if (latestCommandId.current === request.command_id) setError('Não foi possível atualizar as métricas da sessão.');
        }
      }
    } catch {
      if (activeTurn.current?.request.command_id !== request.command_id) return;
      activeTurn.current = null;
      setStatus('error');
      setError('Não foi possível enviar a mensagem. Tente novamente.');
    }
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const text = draft.trim();
    if (!text || busy) return;
    void beginTurn(text, true);
  }

  async function cancel() {
    const turn = activeTurn.current;
    if (!turn || !busy) return;
    setStatus('cancelling');
    try {
      await transport.cancel({
        command_id: turn.request.command_id,
        session_id: session.session_id,
        caller: session.caller,
      });
    } catch {
      setStatus('error');
      setError('Não foi possível cancelar a geração.');
    }
  }

  function retry() {
    if (lastText && !busy) void beginTurn(lastText, false);
  }

  return (
    <main className="chat-page" aria-label="Chat da sessão" aria-busy={busy}>
      <header className="chat-header">
        <div>
          <p className="chat-eyebrow">Sessão single-agent</p>
          <h1>Chat</h1>
        </div>
        <div className="chat-header-meta">
          {resolvedIndicator && <ProviderIndicator data={resolvedIndicator} />}
          <span className={`chat-status chat-status-${status}`} role="status">
            {statusLabel(status)}
          </span>
        </div>
      </header>

      {error && <p className="chat-error" role="alert">{error}</p>}
      {resolvedUsage && <UsageSummary usage={resolvedUsage} />}
      {toolCalls.length > 0 && (
        <section className="chat-tool-calls" aria-label="Chamadas de ferramentas">
          {toolCalls.map((call) => (
            <ToolCallCard
              key={call.id}
              call={call}
              onApprove={onApproveToolCall ? () => onApproveToolCall(call) : undefined}
            />
          ))}
        </section>
      )}

      <ol className="chat-messages" aria-live="polite" aria-label="Mensagens da sessão">
        {messages.length === 0 && <li className="chat-empty">Envie uma mensagem para iniciar a sessão.</li>}
        {messages.map((message) => (
          <li key={message.id} className={`chat-message chat-message-${message.role}`}>
            <span className="chat-message-role">{message.role === 'user' ? 'Você' : 'Agente'}</span>
            <div className="chat-message-content">
              <SafeMarkdown source={message.text || (message.role === 'assistant' && busy ? '…' : '')} />
            </div>
          </li>
        ))}
      </ol>

      <form className="chat-composer" onSubmit={submit}>
        <label htmlFor="chat-message">Mensagem</label>
        <textarea
          id="chat-message"
          name="message"
          value={draft}
          maxLength={65_536}
          rows={4}
          disabled={busy}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Escreva uma mensagem…"
        />
        <div className="chat-actions">
          <button type="submit" disabled={busy || draft.trim().length === 0}>Enviar mensagem</button>
          <button type="button" disabled={!busy} onClick={() => void cancel()}>Cancelar geração</button>
          {status === 'error' && <button type="button" onClick={retry}>Tentar novamente</button>}
        </div>
      </form>
    </main>
  );
}

function ensureAssistant(setMessages: Dispatch<SetStateAction<RenderedMessage[]>>, id: string) {
  setMessages((current) => current.some((message) => message.id === id)
    ? current
    : trimMessages([...current, { id, role: 'assistant', text: '' }]));
}

function appendAssistant(
  setMessages: Dispatch<SetStateAction<RenderedMessage[]>>,
  id: string,
  text: string,
) {
  setMessages((current) => {
    if (!current.some((message) => message.id === id)) {
      return trimMessages([...current, { id, role: 'assistant', text }]);
    }
    return current.map((message) => message.id === id ? { ...message, text: message.text + text } : message);
  });
}

function trimMessages(messages: RenderedMessage[]): RenderedMessage[] {
  return messages.length > MAX_MESSAGES ? messages.slice(-MAX_MESSAGES) : messages;
}

function statusLabel(status: ChatStatus): string {
  switch (status) {
    case 'sending': return 'Enviando…';
    case 'streaming': return 'Gerando…';
    case 'cancelling': return 'Cancelando…';
    case 'completed': return 'Concluída';
    case 'cancelled': return 'Cancelada';
    case 'error': return 'Erro';
    default: return 'Pronta';
  }
}

function defaultIds(): { command_id: string; stream_id: string } {
  const suffix = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return { command_id: `command-${suffix}`, stream_id: `stream-${suffix}` };
}

function isChatSendResult(value: ChatSendResult | void): value is ChatSendResult {
  return Boolean(value)
    && typeof value === 'object'
    && typeof value.provider_id === 'string'
    && typeof value.model_id === 'string'
    && (value.provider_state === 'selected'
      || value.provider_state === 'fallback'
      || value.provider_state === 'unknown'
      || value.provider_state === 'unavailable'
      || value.provider_state === 'degraded')
    && (value.capability === 'confirmed'
      || value.capability === 'unknown'
      || value.capability === 'unsupported')
    && Number.isInteger(value.attempt_number)
    && value.attempt_number > 0;
}
