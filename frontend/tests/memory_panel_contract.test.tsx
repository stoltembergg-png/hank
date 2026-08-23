import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { DesktopMemoryApiClient, MemoryApiClient } from '../src/api/memory';
import { MemoryPanel } from '../src/components/MemoryPanel';
import { MemorySummary } from '../src/types/memory';

const memory: MemorySummary = {
  id: 'mem_1',
  project_id: 'project-a',
  agent_id: 'agent-a',
  memory_type: 'preference',
  status: 'approved',
  content: '<script>alert("xss")</script> api_key=secret123 keep this bounded',
  summary: 'User preference',
  importance: 0.9,
  provenance: 'user_input',
  confidence: 0.95,
  trace_id: 'trace_1',
  created_at: '2026-08-23T00:00:00.000Z',
  updated_at: '2026-08-23T00:00:00.000Z',
};

describe('Memory UI contract PR-132', () => {
  // @spec:AC-769 @spec:AC-772
  it('uses the bridge with project identity and has a safe browser fallback', async () => {
    const client = new DesktopMemoryApiClient();
    await expect(client.list({ project_id: 'project-a' })).resolves.toMatchObject({
      memories: [],
      project_id: 'project-a',
    });

    let command = '';
    let args: unknown;
    const previousWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = {
      __TAURI_INTERNALS__: {
        invoke: async (name: string, input?: Record<string, unknown>): Promise<unknown> => {
          command = name;
          args = input;
          return { project_id: 'project-a', memories: [memory] };
        },
      },
    };
    try {
      const result = await client.list({ project_id: 'project-a', status: 'approved' });
      expect(command).toBe('list_memories');
      expect(args).toEqual({ input: { project_id: 'project-a', status: 'approved' } });
      expect(result.memories[0].project_id).toBe('project-a');
    } finally {
      (globalThis as unknown as { window: unknown }).window = previousWindow;
    }
  });

  it('sends only the typed, confirmed mutation envelope through the desktop bridge', async () => {
    const client = new DesktopMemoryApiClient();
    let command = '';
    let args: unknown;
    const previousWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = {
      __TAURI_INTERNALS__: {
        invoke: async (name: string, input?: Record<string, unknown>): Promise<unknown> => {
          command = name;
          args = input;
          return memory;
        },
      },
    };
    try {
      await client.mutate({
        project_id: 'project-a',
        memory_id: 'mem_1',
        actor_id: 'operator-1',
        trace_id: 'trace-1',
        operation_id: 'operation-1',
        capability: 'memory.write',
        expected_version: 1,
        confirmed: true,
        edit: { kind: 'approve' },
      });
      expect(command).toBe('mutate_memory');
      expect(args).toEqual({
        input: expect.objectContaining({
          project_id: 'project-a',
          memory_id: 'mem_1',
          capability: 'memory.write',
          confirmed: true,
          edit: { kind: 'approve' },
        }),
      });
    } finally {
      (globalThis as unknown as { window: unknown }).window = previousWindow;
    }
  });

  // @spec:AC-770 @spec:AC-771
  it('renders lifecycle, provenance, trace and bounded escaped content without storage access', async () => {
    const apiClient: MemoryApiClient = { list: async () => ({ project_id: 'project-a', memories: [memory] }) };
    render(<MemoryPanel projectId="project-a" apiClient={apiClient} />);
    await screen.findByText('user_input');
    expect(screen.getByText('approved')).toBeTruthy();
    expect(screen.getByText('trace_1')).toBeTruthy();
    expect(screen.getByText('<script>alert("xss")</script> api_key: [REDACTED] keep this bounded')).toBeTruthy();
    expect(document.body.innerHTML).not.toContain('<script>alert');
    expect(document.body.innerHTML).not.toContain('secret123');
  });

  // @spec:AC-771
  it('truncates long previews without changing the stored API contract', async () => {
    const longMemory = { ...memory, content: 'x'.repeat(500) };
    const apiClient: MemoryApiClient = {
      list: async () => ({ project_id: 'project-a', memories: [longMemory] }),
    };
    render(<MemoryPanel projectId="project-a" apiClient={apiClient} />);
    await screen.findByText(/x+…/);
    const preview = document.querySelector('.memory-content');
    expect(preview?.textContent?.length).toBe(321);
  });

  // @spec:AC-770 @spec:AC-772
  it('exposes filters and distinguishes pending from approved content', async () => {
    const pending = { ...memory, id: 'mem_2', status: 'candidate' as const, content: 'pending candidate' };
    const apiClient: MemoryApiClient = {
      list: async () => ({ project_id: 'project-a', memories: [memory, pending] }),
    };
    render(<MemoryPanel projectId="project-a" apiClient={apiClient} />);
    await waitFor(() => expect(screen.getByText('Não ativo (candidate)')).toBeTruthy());
    expect(screen.getByLabelText('Filtrar por status')).toBeTruthy();
    expect(screen.getAllByText('preference')).toHaveLength(2);
  });

  it('requires explicit confirmation before dispatching an approved memory mutation', async () => {
    const candidate = { ...memory, status: 'candidate' as const };
    const mutate = vi.fn().mockResolvedValue(candidate);
    const apiClient = {
      list: async () => ({ project_id: 'project-a', memories: [candidate] }),
      mutate,
    } as unknown as MemoryApiClient;
    const confirmation = vi.spyOn(window, 'confirm').mockReturnValue(false);

    render(<MemoryPanel projectId="project-a" actorId="operator-1" apiClient={apiClient} />);
    await screen.findByText('Não ativo (candidate)');

    fireEvent.click(screen.getByRole('button', { name: 'Aprovar' }));
    expect(confirmation).toHaveBeenCalledWith(expect.stringContaining('aprovação'));
    expect(mutate).not.toHaveBeenCalled();

    confirmation.mockReturnValue(true);
    fireEvent.click(screen.getByRole('button', { name: 'Aprovar' }));
    await waitFor(() => expect(mutate).toHaveBeenCalledWith(expect.objectContaining({
      project_id: 'project-a',
      memory_id: 'mem_1',
      actor_id: 'operator-1',
      confirmed: true,
      edit: { kind: 'approve' },
    })));
    confirmation.mockRestore();
  });

  it('keeps the card state and exposes a visible version conflict after a failed mutation', async () => {
    const mutate = vi.fn().mockRejectedValue(new Error('Concurrency conflict'));
    const apiClient = {
      list: async () => ({ project_id: 'project-a', memories: [memory] }),
      mutate,
    } as unknown as MemoryApiClient;
    vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(<MemoryPanel projectId="project-a" actorId="operator-1" apiClient={apiClient} />);
    await screen.findByText('approved');
    fireEvent.click(screen.getByRole('button', { name: 'Arquivar' }));

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/conflito de versão/i));
    expect(screen.getByText('approved')).toBeTruthy();
    vi.restoreAllMocks();
  });
});
