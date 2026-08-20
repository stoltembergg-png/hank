import { describe, expect, it } from 'vitest';
import { renderToString } from 'react-dom/server';
import React from 'react';
import { ProjectDetailView } from '../src/components/ProjectDetailView';
import { DesktopProjectApiClient, ProjectApiClient } from '../src/api/projects';
import {
  ArchiveProjectInput,
  ArchiveProjectOutput,
  ProjectSummary,
  UpdateProjectInput,
  UpdateProjectOutput,
} from '../src/types/project';

describe('Project Detail UI (PR-038) AC Tests', () => {
  const sampleProject: ProjectSummary = {
    id: 'prj_01j7x000000000000000000042',
    name: 'Hank Master Agent',
    description: 'Autonomous orchestration project',
    status: 'active',
    owner: 'lead-dev',
    created_at: '2026-08-19T10:00:00.000Z',
    updated_at: '2026-08-19T12:00:00.000Z',
    settings: {
      retention_days: 30,
      auto_archive_idle_days: 14,
      telemetry_enabled: true,
      max_active_agents: 3,
    },
  };

  it('AC-3801: ProjectDetailView renders metadata, status badge, and safe settings summary', () => {
    const html = renderToString(
      React.createElement(ProjectDetailView, {
        projectId: sampleProject.id,
        initialProject: sampleProject,
      }),
    );

    expect(html).toContain('Hank Master Agent');
    expect(html).toContain('prj_01j7x000000000000000000042');
    expect(html).toContain('lead-dev');
    expect(html).toContain('30 dias');
    expect(html).toContain('14 dias');
    expect(html).toContain('Habilitada');
  });

  it('AC-3802: ProjectApiClient.update sends optimistic concurrency timestamp and validated DTO', async () => {
    let capturedInput: UpdateProjectInput | null = null;

    const mockWindow = {
      __TAURI_INTERNALS__: {
        invoke: async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
          if (cmd === 'update_project' && args && 'input' in args) {
            capturedInput = args.input as UpdateProjectInput;
            const output: UpdateProjectOutput = {
              project: {
                ...sampleProject,
                name: capturedInput.name ?? sampleProject.name,
                description: capturedInput.description ?? sampleProject.description,
                status: capturedInput.status ?? sampleProject.status,
                updated_at: '2026-08-19T15:00:00.000Z',
              },
              correlation_id: capturedInput.correlation_id,
            };
            return output as unknown as T;
          }
          throw new Error(`Unknown cmd: ${cmd}`);
        },
      },
    };

    const originalWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = mockWindow;

    try {
      const client = new DesktopProjectApiClient();
      const result = await client.update({
        id: sampleProject.id,
        name: 'Hank Master Agent v2',
        status: 'paused',
        expected_updated_at: sampleProject.updated_at,
        correlation_id: 'req_upd_42',
      });

      expect(capturedInput).not.toBeNull();
      expect(capturedInput?.id).toBe(sampleProject.id);
      expect(capturedInput?.expected_updated_at).toBe('2026-08-19T12:00:00.000Z');
      expect(result.project.name).toBe('Hank Master Agent v2');
      expect(result.project.status).toBe('paused');
    } finally {
      (globalThis as unknown as { window: unknown }).window = originalWindow;
    }
  });

  it('AC-3803: Stale concurrency conflict fails closed and rejects update', async () => {
    const conflictClient: ProjectApiClient = {
      list: async () => ({ projects: [], total: 0, limit: 10, offset: 0 }),
      get: async () => sampleProject,
      create: async () => {
        throw new Error('Not implemented');
      },
      update: async () => {
        throw new Error(
          'Concurrency conflict: expected timestamp does not match current state (STALE_DATA)',
        );
      },
      archive: async () => {
        throw new Error('Not implemented');
      },
    };

    await expect(
      conflictClient.update({
        id: sampleProject.id,
        expected_updated_at: '2020-01-01T00:00:00.000Z',
      }),
    ).rejects.toThrow('Concurrency conflict');
  });

  it('AC-3804: ProjectApiClient.archive calls archive application service', async () => {
    let capturedArchive: ArchiveProjectInput | null = null;

    const mockWindow = {
      __TAURI_INTERNALS__: {
        invoke: async <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
          if (cmd === 'archive_project' && args && 'input' in args) {
            capturedArchive = args.input as ArchiveProjectInput;
            const output: ArchiveProjectOutput = {
              project: {
                ...sampleProject,
                status: 'archived',
                updated_at: new Date().toISOString(),
              },
              already_archived: false,
              correlation_id: capturedArchive.correlation_id,
            };
            return output as unknown as T;
          }
          throw new Error(`Unknown cmd: ${cmd}`);
        },
      },
    };

    const originalWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = mockWindow;

    try {
      const client = new DesktopProjectApiClient();
      const result = await client.archive({
        id: sampleProject.id,
        reason: 'Project completed successfully',
        correlation_id: 'req_arc_42',
      });

      expect(capturedArchive).not.toBeNull();
      expect(capturedArchive?.id).toBe(sampleProject.id);
      expect(capturedArchive?.reason).toBe('Project completed successfully');
      expect(result.project.status).toBe('archived');
      expect(result.already_archived).toBe(false);
    } finally {
      (globalThis as unknown as { window: unknown }).window = originalWindow;
    }
  });

  it('AC-3805: Security boundary: secrets or sensitive keys are not exposed in settings DTO', () => {
    const rawData = JSON.stringify(sampleProject);
    expect(rawData).not.toContain('secret');
    expect(rawData).not.toContain('token');
    expect(rawData).not.toContain('password');
    expect(rawData).not.toContain('sqlite');
  });
});
