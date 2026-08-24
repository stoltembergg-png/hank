import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { DesktopSkillApiClient, SkillApiClient } from '../src/api/skills';
import { SkillsPanel } from '../src/components/SkillsPanel';
import { SkillListOutput, SkillSummary } from '../src/types/skill';

const projectId = 'prj_01j7x000000000000000000042';

const projectSkill: SkillSummary = {
  id: 'skill_01j7x000000000000000000101',
  project_id: projectId,
  name: 'reviewer',
  description: '<img src=x onerror=alert(1)> Revê mudanças com segurança.',
  scope: 'project',
  status: 'active',
  version: '1.2.0',
  pinned_version: '1.2.0',
  rollback_version: '1.1.0',
  parent_version: '1.1.0',
  compatibility: 'compatible',
  content_hash: 'a'.repeat(64),
  source: { kind: 'local', reference_digest: 'b'.repeat(64) },
  capabilities: [{ resource: 'project', action: 'read', scope: projectId }],
  policy: { requires_approval: true, allow_runtime_mutation: false, allow_instruction_override: false },
  budget: { max_tokens: 10000, max_cost_micro_usd: 100000, max_parallel_invocations: 2, max_wall_time_seconds: 60, reset_period: 'never' },
  trace_id: 'trace_skill_1',
  revision: 3,
  binding: {
    project_id: projectId,
    scope: 'project',
    current_version: '1.2.0',
    previous_version: '1.1.0',
    import_reference: null,
    enabled: true,
    approval_id: 'approval_1',
    trace_id: 'trace_binding_1',
    revision: 7,
  },
  versions: [
    { version: '1.1.0', status: 'deprecated', compatibility: 'initial', content_hash: 'c'.repeat(64), parent_version: null, created_at: '2026-08-20T00:00:00.000Z' },
    { version: '1.2.0', status: 'active', compatibility: 'compatible', content_hash: 'a'.repeat(64), parent_version: '1.1.0', created_at: '2026-08-23T00:00:00.000Z' },
  ],
};

const globalSkill: SkillSummary = {
  ...projectSkill,
  id: 'skill_01j7x000000000000000000102',
  project_id: null,
  name: 'global-reviewer',
  scope: 'global',
  binding: null,
};

function result(scope: 'project' | 'global', skills: SkillSummary[]): SkillListOutput {
  return { project_id: projectId, scope, skills, total: skills.length, limit: 50, offset: 0, available: true };
}

describe('Skills UI contract PR-144', () => {
  it('calls the typed desktop bridge with project and scope and has a safe unavailable fallback', async () => {
    let command = '';
    let args: unknown;
    const previousWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = {
      __TAURI_INTERNALS__: {
        invoke: async (name: string, input?: Record<string, unknown>): Promise<unknown> => {
          command = name;
          args = input;
          return result('project', []);
        },
      },
    };

    try {
      const client = new DesktopSkillApiClient();
      await expect(client.list({ project_id: projectId, scope: 'project', limit: 50 })).resolves.toMatchObject({
        project_id: projectId,
        scope: 'project',
      });
      expect(command).toBe('list_skills');
      expect(args).toEqual({ input: { project_id: projectId, scope: 'project', limit: 50, offset: 0 } });
    } finally {
      (globalThis as unknown as { window: unknown }).window = previousWindow;
    }

    await expect(new DesktopSkillApiClient().list({ project_id: projectId })).resolves.toMatchObject({
      project_id: projectId,
      available: false,
      skills: [],
    });
  });

  it('sends confirmed rollback envelopes only through the desktop API command', async () => {
    let command = '';
    let args: unknown;
    const previousWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = {
      __TAURI_INTERNALS__: {
        invoke: async (name: string, input?: Record<string, unknown>): Promise<unknown> => {
          command = name;
          args = input;
          return projectSkill;
        },
      },
    };

    try {
      await expect(new DesktopSkillApiClient().rollback?.({
        project_id: projectId,
        skill_id: projectSkill.id,
        actor_id: 'operator-1',
        trace_id: 'trace_binding_1',
        expected_revision: 7,
        approval_id: 'approval_1',
        capability: 'skill.rollback',
        confirmed: true,
      })).resolves.toEqual(projectSkill);
      expect(command).toBe('rollback_skill');
      expect(args).toEqual({ input: expect.objectContaining({
        project_id: projectId,
        skill_id: projectSkill.id,
        capability: 'skill.rollback',
        confirmed: true,
        expected_revision: 7,
      }) });
    } finally {
      (globalThis as unknown as { window: unknown }).window = previousWindow;
    }
  });

  it('renders lifecycle, pinned version, history, binding policy and provenance as inert text', async () => {
    const apiClient: SkillApiClient = {
      list: async ({ scope }) => result(scope ?? 'project', scope === 'global' ? [globalSkill] : [projectSkill]),
      rollback: async () => projectSkill,
    };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);

    await screen.findByText('reviewer');
    expect(screen.getByText('active')).toBeTruthy();
    expect(screen.getByText('Versão ativa').nextElementSibling).toHaveTextContent('1.2.0');
    expect(screen.getByText('1.1.0')).toBeTruthy();
    expect(screen.getByText('project · local')).toBeTruthy();
    expect(screen.getByText(/Aprovação obrigatória/)).toBeTruthy();
    expect(screen.getByText(/Capabilities:/).parentElement).toHaveTextContent('project:read');
    expect(screen.getByText('trace_binding_1')).toBeTruthy();
    expect(screen.getByText('<img src=x onerror=alert(1)> Revê mudanças com segurança.')).toBeTruthy();
    expect(document.body.innerHTML).not.toContain('<img');
  });

  it('ignores a stale response after the selected scope changes', async () => {
    let resolveProject: ((value: SkillListOutput) => void) | undefined;
    const projectRequest = new Promise<SkillListOutput>((resolve) => {
      resolveProject = resolve;
    });
    const globalRequest = Promise.resolve(result('global', [globalSkill]));
    const apiClient: SkillApiClient = {
      list: ({ scope }) => scope === 'global' ? globalRequest : projectRequest,
    };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);

    fireEvent.click(screen.getByRole('tab', { name: 'Globais' }));
    await screen.findByText('global-reviewer');

    resolveProject?.(result('project', [projectSkill]));
    await waitFor(() => expect(screen.queryByText('reviewer')).toBeNull());
    expect(screen.getByText('global-reviewer')).toBeTruthy();
  });

  it('rejects invalid nested bindings and exposes the complete policy context', async () => {
    const invalidBinding = {
      ...projectSkill,
      binding: { ...projectSkill.binding!, project_id: 'other-project' },
    };
    const apiClient: SkillApiClient = {
      list: async () => result('project', [invalidBinding]),
    };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/fora do projeto/i));
    expect(screen.queryByText('reviewer')).toBeNull();
  });

  it('shows bounded approval, budget and explicit import metadata', async () => {
    const importedGlobal: SkillSummary = {
      ...globalSkill,
      binding: {
        ...projectSkill.binding!,
        scope: 'global',
        current_version: globalSkill.version,
        import_reference: 'project-import:global-reviewer',
      },
    };
    const apiClient: SkillApiClient = {
      list: async ({ scope }) => result(scope ?? 'project', scope === 'global' ? [importedGlobal] : []),
    };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);
    fireEvent.click(screen.getByRole('tab', { name: 'Globais' }));

    await screen.findByText('global-reviewer');
    expect(screen.getByText(/Aprovação obrigatória · approval_1/)).toBeTruthy();
    expect(screen.getByText(/100\.000 micro-USD.*60s.*nunca/)).toBeTruthy();
    expect(screen.getByText(/Importação explícita · versão 1.2.0/)).toBeTruthy();
  });

  it('does not render a mutation action for a read-only API client', async () => {
    const apiClient: SkillApiClient = { list: async () => result('project', [projectSkill]) };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);

    await screen.findByText('reviewer');
    expect(screen.queryByRole('button', { name: 'Rollback reviewer' })).toBeNull();
  });

  it('never renders a skill returned for another project', async () => {
    const apiClient: SkillApiClient = {
      list: async () => result('project', [{ ...projectSkill, project_id: 'other-project' }]),
    };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/fora do projeto/i));
    expect(screen.queryByText('reviewer')).toBeNull();
  });

  it('filters the bounded result set by skill name without changing the API scope', async () => {
    const writer = { ...projectSkill, id: 'skill_writer', name: 'writer' };
    const apiClient: SkillApiClient = { list: async () => result('project', [projectSkill, writer]) };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);

    await screen.findByText('reviewer');
    fireEvent.change(screen.getByRole('searchbox', { name: 'Buscar skills' }), { target: { value: 'writer' } });
    expect(screen.getByText('writer')).toBeTruthy();
    expect(screen.queryByText('reviewer')).toBeNull();
  });

  it('renders no more than the bounded page size from an oversized API response', async () => {
    const skills = Array.from({ length: 60 }, (_, index) => ({
      ...projectSkill,
      id: `skill_${index}`,
      name: `skill-${index}`,
      binding: { ...projectSkill.binding!, revision: index + 1 },
    }));
    const apiClient: SkillApiClient = { list: async () => result('project', skills) };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);

    await screen.findByText('skill-0');
    expect(document.querySelectorAll('.skill-card')).toHaveLength(50);
  });

  it('marks an unimported global skill unavailable and sends confirmed rollback context', async () => {
    const rollback = vi.fn().mockResolvedValue(projectSkill);
    const confirmation = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const apiClient: SkillApiClient = {
      list: async ({ scope }) => result(scope ?? 'project', scope === 'global' ? [globalSkill] : [projectSkill]),
      rollback,
    };
    render(<SkillsPanel projectId={projectId} actorId="operator-1" apiClient={apiClient} />);

    await screen.findByText('reviewer');
    fireEvent.click(screen.getByRole('tab', { name: 'Globais' }));
    await screen.findByText('global-reviewer');
    expect(screen.getByText('Indisponível: importe explicitamente para este projeto.')).toBeTruthy();
    expect(screen.queryByRole('button', { name: /rollback global-reviewer/i })).toBeNull();

    fireEvent.click(screen.getByRole('tab', { name: 'Do projeto' }));
    await screen.findByText('reviewer');
    fireEvent.click(screen.getByRole('button', { name: 'Rollback reviewer' }));
    expect(confirmation).toHaveBeenCalledWith(expect.stringContaining('Rollback da skill reviewer'));
    await waitFor(() => expect(rollback).toHaveBeenCalledWith(expect.objectContaining({
      project_id: projectId,
      skill_id: projectSkill.id,
      actor_id: 'operator-1',
      expected_revision: 7,
      capability: 'skill.rollback',
      confirmed: true,
    })));
    confirmation.mockRestore();
  });

  it('shows loading, unavailable and bounded empty states without exposing storage details', async () => {
    const apiClient: SkillApiClient = {
      list: async () => ({ project_id: projectId, scope: 'project', skills: [], total: 0, limit: 50, offset: 0, available: false }),
    };
    render(<SkillsPanel projectId={projectId} apiClient={apiClient} />);
    await screen.findByRole('status');
    expect(screen.getByText(/serviço de skills indisponível/i)).toBeTruthy();
    expect(document.body.innerHTML).not.toMatch(/sqlite|sqlx|password|secret|token/i);
  });
});
