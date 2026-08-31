import { expect, test, type Page } from '@playwright/test';

async function backgroundLuminance(page: Page, selector: string): Promise<number> {
  return page.locator(selector).evaluate((element) => {
    const color = getComputedStyle(element).backgroundColor;
    const channels = color.match(/\d+(?:\.\d+)?/g)?.slice(0, 3).map(Number);

    if (!channels || channels.length !== 3) {
      throw new Error(`Expected an RGB background color for ${selector}, received ${color}.`);
    }

    const [red, green, blue] = channels;
    return red * 0.2126 + green * 0.7152 + blue * 0.0722;
  });
}

async function backgroundColor(page: Page, selector: string): Promise<string> {
  return page.locator(selector).evaluate((element) => getComputedStyle(element).backgroundColor);
}

test('desktop frontend renders the project workspace without a Tauri bridge failure', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Workspace' })).toBeVisible();
  await expect(page.getByText('Hank Desktop', { exact: true })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Gerenciamento de Projetos' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Abrir formulário de criação de projeto' })).toBeVisible();

  await page.getByRole('button', { name: 'Abrir formulário de criação de projeto' }).click();
  await expect(page.getByRole('form', { name: 'Criar novo projeto' })).toBeVisible();
});

test('desktop frontend exposes a scoped workflow draft without claiming persistence', async ({ page }) => {
  await page.addInitScript(() => {
    const project = {
      id: 'project-workflow-surface',
      name: 'Workflow Surface Project',
      description: 'Project with a bounded workflow draft surface',
      status: 'active',
      owner: 'e2e',
      created_at: '2026-08-30T00:00:00.000Z',
      updated_at: '2026-08-30T00:00:00.000Z',
      settings: {
        retention_days: 30,
        auto_archive_idle_days: null,
        telemetry_enabled: false,
        max_active_agents: 3,
      },
    };

    (window as unknown as {
      __TAURI_INTERNALS__: { invoke: (command: string) => Promise<unknown> };
    }).__TAURI_INTERNALS__ = {
      invoke: async (command) => {
        if (command === 'frontend_ready') return { stage: 'APPLICATION_READY' };
        if (command === 'list_projects') return { projects: [project], total: 1, limit: 10, offset: 0 };
        if (command === 'list_memories') return { project_id: project.id, memories: [] };
        if (command === 'list_skills') return { project_id: project.id, scope: 'project', skills: [], total: 0, limit: 50, offset: 0, available: true };
        if (command === 'list_scheduled_jobs') return [];
        throw new Error(`Unexpected command: ${command}`);
      },
    };
  });

  await page.goto('/');
  await page.getByRole('listitem', { name: 'Ver detalhes de Workflow Surface Project' }).click();
  const workflowsButton = page.getByRole('button', { name: 'Workflows', exact: true });
  await expect(workflowsButton).toBeEnabled();
  await workflowsButton.click();

  await expect(page.getByRole('region', { name: 'Workflows do projeto' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Workflow studio' })).toBeVisible();
  await expect(page.getByText('Rascunho local')).toBeVisible();
  await expect(page.getByText('A persistência de workflows ainda não está disponível no desktop.')).toBeVisible();
  expect(await backgroundLuminance(page, '.workflow-surface')).toBeLessThan(95);
  expect(await backgroundLuminance(page, '.workflow-canvas')).toBeLessThan(95);

  await page.getByRole('button', { name: 'Adicionar nó Agent' }).click();
  await expect(page.getByRole('listitem', { name: 'Agent 1' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Salvar workflow' })).toBeDisabled();

  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload();
  await page.getByRole('listitem', { name: 'Ver detalhes de Workflow Surface Project' }).click();
  await page.getByRole('button', { name: 'Workflows', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Workflows do projeto' })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});

test('desktop frontend opens a project session in the read-only workbench', async ({ page }) => {
  await page.addInitScript(() => {
    const project = {
      id: 'project-session-open',
      name: 'Session Open Project',
      description: 'Project with an existing session',
      status: 'active',
      owner: 'e2e',
      created_at: '2026-08-30T00:00:00.000Z',
      updated_at: '2026-08-30T00:00:00.000Z',
      settings: {
        retention_days: 30,
        auto_archive_idle_days: null,
        telemetry_enabled: false,
        max_active_agents: 3,
      },
    };
    const agent = {
      id: 'agent-session-open',
      project_id: project.id,
      name: 'session-agent',
      description: 'Agent for session opening',
      status: 'active',
      personality: {
        name: 'Default',
        description: null,
        traits: ['helpful'],
        communication_style: 'technical',
      },
      created_at: '2026-08-30T00:00:00.000Z',
      updated_at: '2026-08-30T00:00:00.000Z',
    };
    const session = {
      id: 'session-session-open',
      project_id: project.id,
      agent_id: agent.id,
      status: 'active',
      title: 'Open this conversation',
      message_count: 0,
      token_count: 0,
      created_at: '2026-08-30T00:00:00.000Z',
      updated_at: '2026-08-30T00:00:00.000Z',
      closed_at: null,
    };

    (window as unknown as {
      __TAURI_INTERNALS__: { invoke: (command: string) => Promise<unknown> };
    }).__TAURI_INTERNALS__ = {
      invoke: async (command) => {
        if (command === 'frontend_ready') return { stage: 'APPLICATION_READY' };
        if (command === 'list_projects') return { projects: [project], total: 1, limit: 10, offset: 0 };
        if (command === 'list_agents') return { agents: [agent], total: 1, limit: 10, offset: 0 };
        if (command === 'list_sessions') return { sessions: [session], total: 1, limit: 10, offset: 0 };
        if (command === 'list_memories') return { project_id: project.id, memories: [] };
        if (command === 'list_skills') return { project_id: project.id, scope: 'project', skills: [], total: 0, limit: 50, offset: 0, available: true };
        if (command === 'list_scheduled_jobs') return [];
        throw new Error(`Unexpected command: ${command}`);
      },
    };
  });

  await page.goto('/');
  await page.getByRole('listitem', { name: 'Ver detalhes de Session Open Project' }).click();
  expect(await backgroundColor(page, '.project-detail-body .detail-section:nth-of-type(1)')).toBe('rgb(17, 25, 37)');
  expect(await backgroundColor(page, '.project-detail-body .detail-section:nth-of-type(2)')).toBe('rgb(17, 25, 37)');
  await page.getByLabel('Navegação principal').getByRole('button', { name: 'Agents' }).click();
  await expect(page.getByLabel('Navegação principal').getByRole('button', { name: 'Agents' })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByText('session-agent')).toBeVisible();
  expect(await backgroundLuminance(page, '.agent-list-table thead')).toBeLessThan(95);
  await page.getByRole('button', { name: 'Abrir conversas de session-agent' }).click();
  expect(await backgroundLuminance(page, '.session-list-container')).toBeLessThan(95);
  expect(await backgroundLuminance(page, '.session-card')).toBeLessThan(95);
  await page.getByRole('button', { name: 'Abrir conversa' }).click();

  await expect(page.getByRole('heading', { name: 'Open this conversation' })).toBeVisible();
  await expect(page.getByText('Envio de mensagens ainda não está integrado ao desktop.')).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Mensagem' })).toBeDisabled();
  await page.getByRole('button', { name: 'Voltar para conversas' }).click();
  await expect(page.getByRole('button', { name: 'Abrir conversa' })).toBeVisible();
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
  await expect(page.getByRole('region', { name: 'Memórias do projeto' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Skills do projeto' })).toBeVisible();
  expect(await backgroundLuminance(page, '.memory-panel')).toBeLessThan(95);
  expect(await backgroundLuminance(page, '.skills-panel')).toBeLessThan(95);
  expect(await backgroundLuminance(page, '.skill-card')).toBeLessThan(95);
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
  expect(await backgroundLuminance(page, '.automation-list')).toBeGreaterThan(10);
  expect(await backgroundLuminance(page, '.automation-list form')).toBeGreaterThan(10);
  expect(await backgroundLuminance(page, '.automation-list ul')).toBeGreaterThan(10);
  await expect(page.getByText('job-a')).toBeVisible();
  await page.getByRole('button', { name: 'Pausar' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'pausada' })).toBeVisible();
});
