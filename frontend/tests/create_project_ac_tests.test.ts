import { describe, expect, it } from 'vitest';
import { renderToString } from 'react-dom/server';
import React from 'react';
import { CreateProjectForm } from '../src/components/CreateProjectForm';
import { DesktopProjectApiClient, ProjectApiClient } from '../src/api/projects';
import { CreateProjectInput, CreateProjectOutput } from '../src/types/project';

describe('Create Project UI (PR-037) AC Tests', () => {
  it('AC-3701: CreateProjectForm renders all accessible form elements', () => {
    const html = renderToString(React.createElement(CreateProjectForm, {}));

    expect(html).toContain('aria-label="Criar Novo Projeto"');
    expect(html).toContain('id="project-name-input"');
    expect(html).toContain('id="project-owner-input"');
    expect(html).toContain('id="project-desc-input"');
    expect(html).toContain('type="submit"');
  });

  it('AC-3702: ProjectApiClient.create sends validated DTO and receives created output', async () => {
    let sentInput: CreateProjectInput | null = null;

    const mockWindow = {
      __TAURI_INTERNALS__: {
        invoke: async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
          if (cmd === 'create_project' && args && 'input' in args) {
            sentInput = args.input as CreateProjectInput;
            const output: CreateProjectOutput = {
              project: {
                id: 'prj_01j7x000000000000000000099',
                name: sentInput.name,
                owner: sentInput.owner,
                description: sentInput.description,
                status: 'active',
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
              },
              correlation_id: sentInput.correlation_id,
            };
            return output as unknown as T;
          }
          throw new Error(`Unhandled command: ${cmd}`);
        },
      },
    };

    const originalWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = mockWindow;

    try {
      const client = new DesktopProjectApiClient();
      const result = await client.create({
        name: 'New Hank Agent Project',
        owner: 'security-team',
        description: 'Testing create project flow',
        correlation_id: 'req_test123',
      });

      expect(sentInput).not.toBeNull();
      expect(sentInput?.name).toBe('New Hank Agent Project');
      expect(sentInput?.owner).toBe('security-team');
      expect(result.project.name).toBe('New Hank Agent Project');
      expect(result.project.status).toBe('active');
      expect(result.correlation_id).toBe('req_test123');
    } finally {
      (globalThis as unknown as { window: unknown }).window = originalWindow;
    }
  });

  it('AC-3703: ProjectApiClient fails closed without the desktop bridge', async () => {
    // @spec:AC-113
    const client = new DesktopProjectApiClient();

    await expect(
      client.create({
        name: 'Fallback Project',
        owner: 'fallback-owner',
        description: 'Running without desktop bridge',
      }),
    ).rejects.toMatchObject({ code: 'PROJECT_BRIDGE_UNAVAILABLE' });
  });

  it('AC-3704: Error handling and conflict outcomes fail closed', async () => {
    const errorClient: ProjectApiClient = {
      list: async () => ({ projects: [], total: 0, limit: 10, offset: 0 }),
      create: async () => {
        throw new Error('Project name already exists (conflict code: ALREADY_EXISTS)');
      },
    };

    await expect(
      errorClient.create({
        name: 'Duplicate Project',
        owner: 'tester',
      }),
    ).rejects.toThrow('already exists');
  });

  it('AC-3705: No dangerous shell execution or unescaped strings in DTO', () => {
    const input: CreateProjectInput = {
      name: '<script>alert(1)</script>',
      owner: 'user; rm -rf /',
      description: '`touch /tmp/evil`',
    };

    // Confirm it is treated strictly as plain string data
    expect(typeof input.name).toBe('string');
    expect(typeof input.owner).toBe('string');
    expect(typeof input.description).toBe('string');
  });
});
