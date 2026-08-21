import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChatPage, type ChatTransport } from '@/chat/ChatPage';
import type { ChatStreamEvent, ChatStreamSubscription } from '@/contracts/chat-stream';
import type { ToolCallViewModel } from '@/chat/tool-call/ToolCallCard';

const session: Omit<ChatStreamSubscription, 'stream_id' | 'command_id'> = {
  caller: { caller_id: 'caller-1', class: 'desktop' },
  project_id: 'proj-00000000-0000-4000-8000-000000000001',
  agent_id: 'agent-00000000-0000-4000-8000-000000000002',
  session_id: 'sess-00000000-0000-4000-8000-000000000003',
  generation: 1,
};

function makeTransport(): ChatTransport & {
  emit: (event: ChatStreamEvent) => void;
  sendMock: ReturnType<typeof vi.fn>;
  cancelMock: ReturnType<typeof vi.fn>;
} {
  const listeners = new Set<(event: unknown) => void>();
  const sendMock = vi.fn().mockResolvedValue(undefined);
  const cancelMock = vi.fn().mockResolvedValue(undefined);
  return {
    send: sendMock,
    cancel: cancelMock,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    emit: (event) => listeners.forEach((listener) => listener(event)),
    sendMock,
    cancelMock,
  };
}

function event(
  commandId: string,
  streamId: string,
  sequence: number,
  payload: ChatStreamEvent['payload'],
  overrides: Partial<ChatStreamEvent> = {},
): ChatStreamEvent {
  return {
    schema_version: 1,
    ...session,
    command_id: commandId,
    stream_id: streamId,
    sequence,
    payload,
    ...overrides,
  };
}

describe('ChatPage', () => {
  // @spec:AC-662 @spec:AC-664
  it('renders tool calls as scoped read-only cards without executing them', () => {
    const transport = makeTransport();
    const toolCall: ToolCallViewModel = {
      id: 'tool-call-1',
      projectId: session.project_id,
      agentId: session.agent_id,
      toolName: 'git_status',
      toolVersion: '1.0.0',
      traceId: 'trace-1',
      state: 'denied',
      arguments: { path: 'workspace' },
    };

    render(<ChatPage session={session} transport={transport} toolCalls={[toolCall]} />);

    const card = screen.getByRole('article', { name: 'Tool call git_status' });
    expect(card).toBeInTheDocument();
    expect(within(card).getByRole('status')).toHaveTextContent('Negada');
    expect(screen.queryByRole('button', { name: 'Executar ferramenta' })).not.toBeInTheDocument();
  });

  it('sends a scoped command and renders ordered streaming assistant output', async () => {
    const transport = makeTransport();
    render(
      <ChatPage
        session={session}
        transport={transport}
        createIds={() => ({ command_id: 'command-1', stream_id: 'stream-1' })}
      />,
    );
    fireEvent.change(screen.getByRole('textbox', { name: 'Mensagem' }), { target: { value: 'hello' } });
    fireEvent.click(screen.getByRole('button', { name: 'Enviar mensagem' }));
    await waitFor(() => expect(transport.sendMock).toHaveBeenCalledWith({
      schema_version: 1,
      ...session,
      command_id: 'command-1',
      stream_id: 'stream-1',
      cancellation_id: 'cancel-command-1',
      text: 'hello',
    }));

    act(() => {
      transport.emit(event('command-1', 'stream-1', 0, { kind: 'start' }));
      transport.emit(event('command-1', 'stream-1', 1, { kind: 'delta', text: 'world' }));
      transport.emit(event('command-1', 'stream-1', 2, { kind: 'finish', reason: 'completed' }));
    });
    expect(screen.getByText('hello')).toBeInTheDocument();
    expect(screen.getByText('world')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent(/Concluída/i);
  });

  it('ignores foreign and stale events without mutating the active assistant message', async () => {
    const transport = makeTransport();
    render(<ChatPage session={session} transport={transport} createIds={() => ({ command_id: 'command-1', stream_id: 'stream-1' })} />);
    fireEvent.change(screen.getByRole('textbox', { name: 'Mensagem' }), { target: { value: 'hello' } });
    fireEvent.click(screen.getByRole('button', { name: 'Enviar mensagem' }));
    act(() => {
      transport.emit(event('command-1', 'stream-1', 0, { kind: 'start' }));
      transport.emit(event('other-command', 'other-stream', 1, { kind: 'delta', text: 'foreign' }));
      transport.emit(event('command-1', 'stream-1', 1, { kind: 'delta', text: 'accepted' }));
    });
    expect(screen.getByText('accepted')).toBeInTheDocument();
    expect(screen.queryByText('foreign')).not.toBeInTheDocument();
  });

  it('cancels the scoped command and exposes an accessible terminal status', async () => {
    const transport = makeTransport();
    render(<ChatPage session={session} transport={transport} createIds={() => ({ command_id: 'command-1', stream_id: 'stream-1' })} />);
    fireEvent.change(screen.getByRole('textbox', { name: 'Mensagem' }), { target: { value: 'hello' } });
    fireEvent.click(screen.getByRole('button', { name: 'Enviar mensagem' }));
    fireEvent.click(screen.getByRole('button', { name: 'Cancelar geração' }));
    await waitFor(() => expect(transport.cancelMock).toHaveBeenCalledWith({
      command_id: 'command-1',
      session_id: session.session_id,
    }));
    act(() => {
      transport.emit(event('command-1', 'stream-1', 0, { kind: 'start' }));
      transport.emit(event('command-1', 'stream-1', 1, { kind: 'cancel', reason: 'user' }));
    });
    expect(screen.getByRole('status')).toHaveTextContent(/Cancelada/i);
  });

  it('shows a redacted error and retries the last command without persisting provider details', async () => {
    const transport = makeTransport();
    transport.sendMock.mockRejectedValueOnce(new Error('provider secret payload'));
    render(<ChatPage session={session} transport={transport} createIds={() => ({ command_id: 'command-1', stream_id: 'stream-1' })} />);
    fireEvent.change(screen.getByRole('textbox', { name: 'Mensagem' }), { target: { value: 'hello' } });
    fireEvent.click(screen.getByRole('button', { name: 'Enviar mensagem' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/Não foi possível enviar/i));
    expect(screen.getByRole('alert')).not.toHaveTextContent(/secret|provider payload/i);
    fireEvent.click(screen.getByRole('button', { name: 'Tentar novamente' }));
    await waitFor(() => expect(transport.sendMock).toHaveBeenCalledTimes(2));
  });

  it('does not send blank messages and exposes labeled chat controls', () => {
    const transport = makeTransport();
    render(<ChatPage session={session} transport={transport} createIds={() => ({ command_id: 'command-1', stream_id: 'stream-1' })} />);
    expect(screen.getByRole('main', { name: 'Chat da sessão' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Mensagem' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Enviar mensagem' })).toBeDisabled();
    fireEvent.change(screen.getByRole('textbox', { name: 'Mensagem' }), { target: { value: '   ' } });
    expect(transport.sendMock).not.toHaveBeenCalled();
  });
});
