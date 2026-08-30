import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  DesktopSessionApiClient,
  SessionBridgeUnavailableError,
} from '@/api/sessions';

describe('Desktop session bridge contract', () => {
  afterEach(() => {
    delete (window as Window & { __TAURI_INVOKE__?: unknown }).__TAURI_INVOKE__;
  });

  it('sends project and agent scope through the typed list command', async () => {
    const invoke = vi.fn().mockResolvedValue({
      sessions: [],
      total: 0,
      limit: 20,
      offset: 0,
      correlation_id: 'corr-list',
    });
    Object.assign(window, { __TAURI_INVOKE__: invoke });

    await new DesktopSessionApiClient().list({
      project_id: 'proj-1',
      agent_id: 'agent-1',
      limit: 20,
      offset: 0,
      correlation_id: 'corr-list',
    });

    expect(invoke).toHaveBeenCalledWith('list_sessions', {
      input: {
        project_id: 'proj-1',
        agent_id: 'agent-1',
        limit: 20,
        offset: 0,
        correlation_id: 'corr-list',
      },
    });
  });

  it('sends only the bounded create session input', async () => {
    const invoke = vi.fn().mockResolvedValue({
      session: {
        id: 'sess-1',
        project_id: 'proj-1',
        agent_id: 'agent-1',
        status: 'active',
        title: 'Release chat',
        message_count: 0,
        token_count: 0,
        created_at: '2026-08-30T00:00:00Z',
        updated_at: '2026-08-30T00:00:00Z',
        closed_at: null,
      },
      correlation_id: 'corr-create',
    });
    Object.assign(window, { __TAURI_INVOKE__: invoke });

    await new DesktopSessionApiClient().create({
      project_id: 'proj-1',
      agent_id: 'agent-1',
      title: 'Release chat',
      correlation_id: 'corr-create',
    });

    expect(invoke).toHaveBeenCalledWith('create_session', {
      input: {
        project_id: 'proj-1',
        agent_id: 'agent-1',
        title: 'Release chat',
        correlation_id: 'corr-create',
      },
    });
  });

  it('fails closed when the Tauri session bridge is unavailable', async () => {
    await expect(
      new DesktopSessionApiClient().list({
        project_id: 'proj-1',
        agent_id: 'agent-1',
      }),
    ).rejects.toBeInstanceOf(SessionBridgeUnavailableError);
  });
});
