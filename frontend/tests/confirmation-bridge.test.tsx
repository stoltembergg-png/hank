import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import {
  ConfirmationCard,
  type ConfirmationRequest,
} from '@/chat/confirmation/ConfirmationCard';
import {
  DesktopConfirmationApiClient,
  type ConfirmationApiClient,
} from '@/api/confirmations';
import {
  isConfirmationEvent,
  type ConfirmationEvent,
} from '@/contracts/confirmation';

const request: ConfirmationRequest = {
  request_id: 'request-1',
  project_id: 'project-1',
  agent_id: 'agent-1',
  tool_name: 'git_commit',
  tool_version: '1.0.0',
  schema_hash: 'schema-hash',
  args_hash: 'args-hash',
  effect: 'write',
  budget_ref: null,
  trace_id: 'trace-1',
  actor_id: 'operator-1',
  policy: 'ask_every_time',
  created_at_ms: 1_000,
  expires_at_ms: 61_000,
};

describe('confirmation bridge contract', () => {
  // @spec:AC-675
  it('invokes typed commands with bounded artifacts only', async () => {
    const invoke = vi.fn()
      .mockResolvedValueOnce(request)
      .mockResolvedValueOnce({ ...request, grant_id: 'grant-1' });
    const client = new DesktopConfirmationApiClient(invoke);

    await client.submit(request);
    await client.approve({ request_id: request.request_id, actor_id: 'operator-1', now_ms: 2_000 });

    expect(invoke).toHaveBeenNthCalledWith(1, 'submit_confirmation_request', { request });
    expect(invoke).toHaveBeenNthCalledWith(2, 'approve_confirmation_request', {
      input: { request_id: 'request-1', actor_id: 'operator-1', now_ms: 2_000 },
    });
    expect(JSON.stringify(invoke.mock.calls)).not.toContain('secret');
  });

  // @spec:AC-676
  it('renders bounded metadata and exposes accessible approval actions', async () => {
    const apiClient: ConfirmationApiClient = {
      approve: vi.fn().mockResolvedValue({ ...request, grant_id: 'grant-1' }),
      revoke: vi.fn().mockResolvedValue(undefined),
    };

    render(<ConfirmationCard request={request} apiClient={apiClient} nowMs={2_000} />);

    expect(screen.getByRole('heading', { name: 'Aprovação necessária' })).toBeInTheDocument();
    expect(screen.getByText('git_commit · versão 1.0.0')).toBeInTheDocument();
    expect(screen.getByText('Hash dos argumentos: args-hash')).toBeInTheDocument();
    expect(screen.queryByText(/schema|secret/i)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Aprovar git_commit' }));
    fireEvent.click(screen.getByRole('button', { name: 'Revogar solicitação' }));

    expect(apiClient.approve).toHaveBeenCalledWith({
      request_id: 'request-1',
      actor_id: 'operator-1',
      now_ms: 2_000,
    });
    expect(apiClient.revoke).toHaveBeenCalledWith(request);
  });

  // @spec:AC-675
  it('accepts only current-schema bounded confirmation events', () => {
    const event: ConfirmationEvent = {
      schema_version: 1,
      event_id: 'event-1',
      request_id: 'request-1',
      sequence: 0,
      payload: { kind: 'request_submitted', request },
    };

    expect(isConfirmationEvent(event)).toBe(true);
    expect(isConfirmationEvent({ ...event, payload: { kind: 'request_submitted', request: { ...request, arguments: 'secret' } } })).toBe(false);
    expect(isConfirmationEvent({ ...event, schema_version: 2 })).toBe(false);
  });
});
