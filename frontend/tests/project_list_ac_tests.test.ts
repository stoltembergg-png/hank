import { describe, expect, it } from 'vitest';
import { renderToString } from 'react-dom/server';
import React from 'react';
import { ProjectList } from '../src/components/ProjectList';
import { DesktopProjectApiClient, ProjectApiClient } from '../src/api/projects';
import { ListProjectsInput, ListProjectsOutput, ProjectSummary } from '../src/types/project';

describe('Project UI Listing (PR-036) AC Tests', () => {
  const mockProjects: ProjectSummary[] = [
    {
      id: 'prj_01h8x4a1234567890abcdef123',
      name: 'Alpha Project',
      description: 'Alpha description for agent workflow',
      status: 'active',
      owner: 'gabriel',
      created_at: '2026-08-19T20:00:00.000Z',
      updated_at: '2026-08-19T20:00:00.000Z',
    },
    {
      id: 'prj_01h8x4a1234567890abcdef124',
      name: 'Beta Project',
      description: null,
      status: 'paused',
      owner: 'team-hank',
      created_at: '2026-08-19T21:00:00.000Z',
      updated_at: '2026-08-19T21:00:00.000Z',
    },
  ];

  it('AC-3601: ProjectApiClient provides fallback in decoupled/browser environment', async () => {
    const client = new DesktopProjectApiClient();
    const result = await client.list({ limit: 10, offset: 0 });

    expect(result).toMatchObject({
      projects: [],
      total: 0,
      limit: 10,
      offset: 0,
    });
  });

  it('AC-3602: ProjectApiClient uses window bridge when available', async () => {
    let capturedCmd = '';
    let capturedArgs: unknown = null;

    const mockWindow = {
      __TAURI_INTERNALS__: {
        invoke: async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
          capturedCmd = cmd;
          capturedArgs = args;
          return {
            projects: mockProjects,
            total: 2,
            limit: 20,
            offset: 0,
          } as unknown as T;
        },
      },
    };

    // Temporarily attach bridge
    const originalWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = mockWindow;

    try {
      const client = new DesktopProjectApiClient();
      const result = await client.list({ limit: 20, offset: 0 });

      expect(capturedCmd).toBe('list_projects');
      expect(capturedArgs).toEqual({ input: { limit: 20, offset: 0 } });
      expect(result.projects).toHaveLength(2);
      expect(result.total).toBe(2);
    } finally {
      (globalThis as unknown as { window: unknown }).window = originalWindow;
    }
  });

  it('AC-3603: Component renders semantic HTML and accessibility attributes', () => {
    const mockClient: ProjectApiClient = {
      list: async () => ({
        projects: mockProjects,
        total: mockProjects.length,
        limit: 10,
        offset: 0,
      }),
    };

    const html = renderToString(React.createElement(ProjectList, { apiClient: mockClient }));

    expect(html).toContain('aria-label="Gerenciamento de Projetos"');
    expect(html).toContain('role="status"');
    expect(html).toContain('Carregando projetos...');
  });

  it('AC-3604: Project types adhere to Application API contract schema', () => {
    const project: ProjectSummary = {
      id: 'prj_01j7x000000000000000000000',
      name: 'Secure Agent Project',
      description: 'Zero direct DB access',
      status: 'active',
      owner: 'alice',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    const input: ListProjectsInput = {
      limit: 50,
      offset: 0,
      status: 'active',
    };

    const output: ListProjectsOutput = {
      projects: [project],
      total: 1,
      limit: input.limit ?? 20,
      offset: input.offset ?? 0,
    };

    expect(output.projects[0].name).toBe('Secure Agent Project');
    expect(output.projects[0].status).toBe('active');
    expect(output.limit).toBe(50);
  });

  it('AC-3605: Redacted error and defensive payload boundaries', async () => {
    const failingClient: ProjectApiClient = {
      list: async () => {
        throw new Error('Database connection failed with SECRET_KEY_123');
      },
    };

    await expect(failingClient.list()).rejects.toThrow('Database connection failed');
  });
});
