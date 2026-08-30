import { expect, test } from '@playwright/test';

test('desktop frontend renders the project workspace without a Tauri bridge failure', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Workspace' })).toBeVisible();
  await expect(page.getByText('Hank Desktop', { exact: true })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Gerenciamento de Projetos' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Abrir formulário de criação de projeto' })).toBeVisible();

  await page.getByRole('button', { name: 'Abrir formulário de criação de projeto' }).click();
  await expect(page.getByRole('form', { name: 'Criar novo projeto' })).toBeVisible();
});

test('desktop frontend renders project-scoped Skills and keeps unimported globals unavailable', async ({ page }) => {
  await page.addInitScript(() => {
    const projectId = 'prj_e2e_skill_project';
    const project = {
      id: projectId,
      name: 'Skill E2E Project',
      description: 'Project-scoped fixture',
      status: 'active',
      owner: 'e2e',
      created_at: '2026-08-24T00:00:00.000Z',
      updated_at: '2026-08-24T00:00:00.000Z',
      settings: {
        retention_days: 30,
        auto_archive_idle_days: null,
        telemetry_enabled: false,
        max_active_agents: 3,
      },
    };
    const projectSkill = {
      id: 'skill_e2e_reviewer',
      project_id: projectId,
      name: 'reviewer',
      description: '<img src=x onerror=alert(1)> Safe description',
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
      trace_id: 'trace_e2e_skill',
      revision: 3,
      binding: {
        project_id: projectId,
        scope: 'project',
        current_version: '1.2.0',
        previous_version: '1.1.0',
        import_reference: null,
        enabled: true,
        approval_id: 'approval_e2e',
        trace_id: 'trace_e2e_binding',
        revision: 7,
      },
      versions: [
        { version: '1.1.0', status: 'deprecated', compatibility: 'initial', content_hash: 'c'.repeat(64), parent_version: null, created_at: '2026-08-20T00:00:00.000Z' },
        { version: '1.2.0', status: 'active', compatibility: 'compatible', content_hash: 'a'.repeat(64), parent_version: '1.1.0', created_at: '2026-08-23T00:00:00.000Z' },
      ],
    };
    const globalSkill = { ...projectSkill, id: 'skill_e2e_global', name: 'global-reviewer', project_id: null, scope: 'global', binding: null };
    const editorDocument = {
      project_id: projectId,
      skill_id: projectSkill.id,
      base_version: projectSkill.version,
      status: 'active',
      revision: projectSkill.revision,
      manifest_json: JSON.stringify({ id: projectSkill.id, version: projectSkill.version, name: projectSkill.name }),
      markdown: '# Instructions\nKeep the active version unchanged.',
      files: [],
      policy: projectSkill.policy,
      budget: projectSkill.budget,
      trace_id: projectSkill.trace_id,
      content_hash: projectSkill.content_hash,
    };

    (window as unknown as { __TAURI_INVOKE__: (command: string, args?: { input?: { scope?: string } }) => Promise<unknown> }).__TAURI_INVOKE__ = async (command, args) => {
      if (command === 'list_projects') return { projects: [project], total: 1, limit: 10, offset: 0 };
      if (command === 'list_memories') return { project_id: projectId, memories: [] };
      if (command === 'list_skills') {
        const scope = args?.input?.scope ?? 'project';
        const skills = scope === 'global' ? [globalSkill] : [projectSkill];
        return { project_id: projectId, scope, skills, total: skills.length, limit: 50, offset: 0, available: true };
      }
      if (command === 'get_skill_editor') return editorDocument;
      if (command === 'validate_skill_draft') return { valid: true, quarantined: false, diagnostics: [], errors: [] };
      if (command === 'save_skill_draft') return { project_id: projectId, skill_id: projectSkill.id, version: '1.3.0', status: 'draft', content_hash: 'd'.repeat(64), changed: true, quarantined: false, revision: projectSkill.revision };
      if (command === 'discard_skill_draft') return { project_id: projectId, skill_id: projectSkill.id, version: '1.3.0', status: 'archived', content_hash: 'd'.repeat(64), changed: true, quarantined: false, revision: projectSkill.revision };
      throw new Error(`Unexpected command: ${command}`);
    };
  });

  await page.goto('/');
  await page.getByRole('listitem', { name: 'Ver detalhes de Skill E2E Project' }).click();
  await expect(page.getByRole('heading', { name: 'reviewer' })).toBeVisible();
  await expect(page.getByText('Versão ativa').locator('..')).toContainText('1.2.0');
  await expect(page.locator('.skill-description')).toContainText('<img src=x onerror=alert(1)> Safe description');
  await expect(page.locator('.skill-description')).not.toHaveCount(0);

  await page.getByRole('button', { name: 'Editar rascunho reviewer' }).click();
  await expect(page.getByRole('heading', { name: 'Editor de skill' })).toBeVisible();
  await page.getByLabel('Instruções Markdown').fill('reviewed draft');
  await page.getByRole('button', { name: 'Validar rascunho' }).click();
  await expect(page.getByRole('alert').filter({ hasText: 'Rascunho válido' })).toBeVisible();
  page.once('dialog', (dialog) => dialog.accept());
  await page.getByRole('button', { name: 'Salvar rascunho' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'salvo' })).toBeVisible();

  await page.getByRole('tab', { name: 'Globais' }).click();
  await expect(page.getByRole('heading', { name: 'global-reviewer' })).toBeVisible();
  await expect(page.getByText('Indisponível: importe explicitamente para este projeto.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Rollback global-reviewer' })).toHaveCount(0);
});

test('desktop frontend renders project-scoped automation controls and explicit pause flow', async ({ page }) => {
  await page.addInitScript(() => {
    const project = { id: 'project-automation', name: 'Automation Project', description: null, status: 'active', owner: 'owner', created_at: '2026-08-24T00:00:00.000Z', updated_at: '2026-08-24T00:00:00.000Z', settings: { retention_days: 30, auto_archive_idle_days: null, telemetry_enabled: false, max_active_agents: 3 } };
    let jobs = [{ project_id: project.id, job_id: 'job-a', owner_id: project.owner, trigger_kind: 'interval', trigger_value: '60', target_kind: 'workflow', target_id: 'workflow-a', target_version: 1, timezone: 'UTC', concurrency_limit: 1, missed_run_policy: 'skip', enabled: true, lifecycle: 'active', revision: 0 }];
    (window as unknown as { __TAURI_INTERNALS__: { invoke: (command: string, args?: { input?: any }) => Promise<unknown> } }).__TAURI_INTERNALS__ = { invoke: async (command, args) => {
      if (command === 'list_projects') return { projects: [project], total: 1, limit: 10, offset: 0 };
      if (command === 'list_memories') return { project_id: project.id, memories: [] };
      if (command === 'list_skills') return { project_id: project.id, scope: 'project', skills: [], total: 0, limit: 50, offset: 0, available: true };
      if (command === 'list_scheduled_jobs') return jobs;
      if (command === 'create_scheduled_job') return args?.input;
      if (command === 'update_scheduled_job') { jobs = [{ ...jobs[0], enabled: false, lifecycle: 'disabled', revision: 1 }]; return jobs[0]; }
      throw new Error(`Unexpected command: ${command}`);
    } };
  });
  await page.goto('/');
  await page.getByRole('listitem', { name: 'Ver detalhes de Automation Project' }).click();
  await expect(page.getByRole('region', { name: 'Automações do projeto' })).toBeVisible();
  await expect(page.getByText('job-a')).toBeVisible();
  await page.getByRole('button', { name: 'Pausar' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'pausada' })).toBeVisible();
});
