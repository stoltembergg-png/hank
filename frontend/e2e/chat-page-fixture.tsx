import React from 'react';
import ReactDOM from 'react-dom/client';
import { ChatPage, type ChatCommandRequest, type ChatTransport } from '../src/chat/ChatPage';
import type { ChatStreamEvent } from '../src/contracts/chat-stream';
import '../src/App.css';

const session = {
  caller: { caller_id: 'fixture-caller', class: 'desktop' },
  project_id: 'project-chat-fixture',
  agent_id: 'agent-chat-fixture',
  session_id: 'session-chat-fixture',
  generation: 1,
} as const;

const listeners = new Set<(event: unknown) => void>();

const transport: ChatTransport = {
  send: async (request: ChatCommandRequest) => {
    const base = {
      schema_version: 1,
      ...session,
      command_id: request.command_id,
      stream_id: request.stream_id,
    };
    const events: ChatStreamEvent[] = [
      { ...base, sequence: 0, payload: { kind: 'start' } },
      { ...base, sequence: 1, payload: { kind: 'delta', text: 'Resposta da fixture.' } },
      { ...base, sequence: 2, payload: { kind: 'finish', reason: 'completed' } },
    ];

    for (const event of events) {
      await Promise.resolve();
      listeners.forEach((listener) => listener(event));
    }
  },
  cancel: async () => undefined,
  subscribe: (listener) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};

ReactDOM.createRoot(document.getElementById('root')!).render(
  <ChatPage
    session={session}
    transport={transport}
    createIds={() => ({ command_id: 'fixture-command', stream_id: 'fixture-stream' })}
  />,
);
