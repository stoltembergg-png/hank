import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import React from 'react';
import { DesktopSkillEditorApiClient, SkillEditorApiClient } from '../src/api/skillEditor';
import { SkillEditor } from '../src/components/SkillEditor';
import { SkillEditorDocument, SkillValidationResult } from '../src/types/skillEditor';

const projectId = 'prj_01j7x000000000000000000042';
const otherProjectId = 'prj_01j7x000000000000000000043';
const skillId = 'skill_01j7x000000000000000000101';

const editorDocument: SkillEditorDocument = {
  project_id: projectId,
  skill_id: skillId,
  base_version: '1.0.0',
  status: 'active',
  revision: 2,
  manifest_json: JSON.stringify({ id: skillId, version: '1.1.0', name: 'reviewer' }),
  markdown: '# Instructions\nKeep changes reviewable.',
  files: [{ path: 'references/checklist.md', role: 'reference', content: 'Review only.' }],
  policy: { requires_approval: true, allow_runtime_mutation: false, allow_instruction_override: false },
  budget: { max_tokens: 10000, max_cost_micro_usd: 100000, max_parallel_invocations: 2, max_wall_time_seconds: 60, reset_period: 'never' },
  trace_id: 'trace_editor_1',
  content_hash: 'a'.repeat(64),
};

const valid: SkillValidationResult = { valid: true, quarantined: false, diagnostics: [], errors: [] };

function apiFor(value: SkillEditorDocument = editorDocument): SkillEditorApiClient {
  return {
    load: async () => value,
    validate: async () => valid,
    saveDraft: async () => ({ project_id: value.project_id, skill_id: value.skill_id, version: '1.1.0', status: 'draft', content_hash: 'b'.repeat(64), changed: true, quarantined: false, revision: value.revision }),
    discardDraft: async () => ({ project_id: value.project_id, skill_id: value.skill_id, version: '1.1.0', status: 'archived', content_hash: 'b'.repeat(64), changed: true, quarantined: false, revision: value.revision }),
  };
}

describe('Skill editor contract PR-145', () => {
  // @spec:AC-787
  it('uses typed governed bridge commands with explicit save and discard envelopes', async () => {
    const commands: string[] = [];
    const payloads: unknown[] = [];
    const previousWindow = globalThis.window;
    (globalThis as unknown as { window: unknown }).window = {
      __TAURI_INTERNALS__: {
        invoke: async (name: string, input?: Record<string, unknown>): Promise<unknown> => {
          commands.push(name);
          payloads.push(input);
          if (name === 'get_skill_editor') return editorDocument;
          if (name === 'validate_skill_draft') return valid;
          return { project_id: projectId, skill_id: skillId, version: '1.1.0', status: 'draft', content_hash: 'b'.repeat(64), changed: true, quarantined: false, revision: 2 };
        },
      },
    };

    try {
      const client = new DesktopSkillEditorApiClient();
      await expect(client.load({ project_id: projectId, skill_id: skillId, version: '1.0.0' })).resolves.toEqual(editorDocument);
      await expect(client.validate({ project_id: projectId, skill_id: skillId, base_version: '1.0.0', document: '---\n{}\n---\n# Body' })).resolves.toEqual(valid);
      await expect(client.saveDraft({
        project_id: projectId,
        skill_id: skillId,
        actor_id: 'operator-1',
        trace_id: 'trace_editor_1',
        expected_revision: 2,
        base_version: '1.0.0',
        document: '---\n{}\n---\n# Body',
        budget: editorDocument.budget,
        policy: { allow: true, max_document_bytes: 65536 },
        capability: 'skill.edit',
        confirmed: true,
      })).resolves.toMatchObject({ status: 'draft' });
      await expect(client.discardDraft({
        project_id: projectId,
        skill_id: skillId,
        actor_id: 'operator-1',
        trace_id: 'trace_editor_1',
        expected_revision: 2,
        version: '1.1.0',
        capability: 'skill.discard',
        confirmed: true,
      })).resolves.toMatchObject({ status: 'draft' });
      expect(commands).toEqual(['get_skill_editor', 'validate_skill_draft', 'save_skill_draft', 'discard_skill_draft']);
      expect(payloads[2]).toEqual({ input: expect.objectContaining({ capability: 'skill.edit', confirmed: true, expected_revision: 2 }) });
      expect(payloads[3]).toEqual({ input: expect.objectContaining({ capability: 'skill.discard', confirmed: true, version: '1.1.0' }) });
    } finally {
      (globalThis as unknown as { window: unknown }).window = previousWindow;
    }
  });

  // @spec:AC-788
  it('validates before save, keeps content inert, and requires explicit confirmation', async () => {
    const saveDraft = vi.fn().mockResolvedValue({ project_id: projectId, skill_id: skillId, version: '1.1.0', status: 'draft', content_hash: 'b'.repeat(64), changed: true, quarantined: false, revision: 2 });
    const validate = vi.fn().mockResolvedValue({ valid: false, quarantined: true, diagnostics: [{ code: 'instruction_override', severity: 'quarantine', line: 3 }], errors: [] });
    const apiClient: SkillEditorApiClient = { ...apiFor(), validate, saveDraft };
    render(<SkillEditor projectId={projectId} skillId={skillId} apiClient={apiClient} />);

    await screen.findByDisplayValue(/Keep changes reviewable/);
    fireEvent.change(screen.getByLabelText('Instruções Markdown'), { target: { value: '<img src=x onerror=alert(1)>' } });
    fireEvent.click(screen.getByRole('button', { name: 'Validar rascunho' }));

    await screen.findByRole('alert');
    expect(screen.getByRole('alert')).toHaveTextContent(/quarentena/i);
    expect(saveDraft).not.toHaveBeenCalled();
    expect(globalThis.document.body.innerHTML).not.toContain('<img');
  });

  // @spec:AC-789
  it('does not persist a draft until the operator confirms the save', async () => {
    const saveDraft = vi.fn().mockResolvedValue({ project_id: projectId, skill_id: skillId, version: '1.1.0', status: 'draft', content_hash: 'b'.repeat(64), changed: true, quarantined: false, revision: 2 });
    const validate = vi.fn().mockResolvedValue(valid);
    const apiClient: SkillEditorApiClient = { ...apiFor(), validate, saveDraft };
    const confirmation = vi.spyOn(window, 'confirm').mockReturnValue(false);
    render(<SkillEditor projectId={projectId} skillId={skillId} apiClient={apiClient} />);

    await screen.findByDisplayValue(/Keep changes reviewable/);
    fireEvent.change(screen.getByLabelText('Instruções Markdown'), { target: { value: 'confirmed draft' } });
    fireEvent.click(screen.getByRole('button', { name: 'Validar rascunho' }));
    await waitFor(() => expect(validate).toHaveBeenCalled());
    fireEvent.click(screen.getByRole('button', { name: 'Salvar rascunho' }));

    await waitFor(() => expect(confirmation).toHaveBeenCalled());
    expect(saveDraft).not.toHaveBeenCalled();
    confirmation.mockReturnValue(true);
    fireEvent.click(screen.getByRole('button', { name: 'Salvar rascunho' }));
    await waitFor(() => expect(saveDraft).toHaveBeenCalled());
    confirmation.mockRestore();
  });

  // @spec:AC-790
  it('discards in-memory edits when the selected project changes', async () => {
    const nextDocument = { ...editorDocument, project_id: otherProjectId, markdown: '# Other project content' };
    const apiClient: SkillEditorApiClient = {
      ...apiFor(),
      load: vi.fn().mockImplementation(({ project_id }) => Promise.resolve(project_id === projectId ? editorDocument : nextDocument)),
    };
    const { rerender } = render(<SkillEditor projectId={projectId} skillId={skillId} apiClient={apiClient} />);
    await screen.findByDisplayValue(/Keep changes reviewable/);
    fireEvent.change(screen.getByLabelText('Instruções Markdown'), { target: { value: 'local unsaved content' } });

    rerender(<SkillEditor projectId={otherProjectId} skillId={skillId} apiClient={apiClient} />);
    await screen.findByDisplayValue(/Other project content/);
    await waitFor(() => expect(screen.queryByDisplayValue('local unsaved content')).toBeNull());
  });
});
