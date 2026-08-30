import { createHash } from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const binary = process.env.HANK_DESKTOP_BIN;
const dataDir = process.env.HANK_E2E_APP_DATA_DIR;
const diagnostics = process.env.HANK_DESKTOP_E2E_ARTIFACTS ?? path.resolve('reports');
const base = `http://127.0.0.1:${process.env.HANK_WEBDRIVER_PORT ?? 4444}`;
const projectName = 'Hank E2E Project';
const updatedName = 'Hank E2E Project Updated';
const owner = 'ci@hank.local';
const description = 'Desktop Project Lifecycle E2E';
const updatedDescription = 'Desktop Project Lifecycle E2E Updated';

if (!binary || !dataDir) throw new Error('HANK_DESKTOP_BIN and HANK_E2E_APP_DATA_DIR are required');
await fs.mkdir(diagnostics, { recursive: true });

const artifactBytes = await fs.readFile(binary);
const artifactIdentity = {
  executable: path.resolve(binary),
  sha256: createHash('sha256').update(artifactBytes).digest('hex'),
  version: process.env.HANK_EXPECTED_VERSION ?? null,
  commitSha: process.env.HANK_EXPECTED_COMMIT_SHA ?? process.env.GITHUB_SHA ?? null,
  treeSha: process.env.HANK_EXPECTED_TREE_SHA ?? null,
  buildTimestamp: process.env.HANK_BUILD_TIMESTAMP ?? (await fs.stat(binary)).mtime.toISOString(),
  platform: process.platform,
};
await fs.writeFile(path.join(diagnostics, 'artifact-identity.json'), JSON.stringify(artifactIdentity, null, 2));

class WebDriverSession {
  constructor() { this.sessionId = undefined; }
  async request(method, route, body) {
    const response = await fetch(`${base}${route}`, {
      method,
      headers: { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const payload = await response.json();
    if (!response.ok || payload.value?.error) {
      throw new Error(`WebDriver ${method} ${route}: ${JSON.stringify(payload.value ?? payload)}`);
    }
    return payload.value;
  }
  async start() {
    const value = await this.request('POST', '/session', {
      capabilities: { alwaysMatch: { browserName: 'wry', 'tauri:options': { application: path.resolve(binary) } } },
    });
    this.sessionId = value.sessionId;
    return this;
  }
  async end() {
    if (this.sessionId) await this.request('DELETE', `/session/${this.sessionId}`);
    this.sessionId = undefined;
  }
  async find(selector) {
    const value = await this.request('POST', `/session/${this.sessionId}/element`, { using: 'css selector', value: selector });
    return value['element-6066-11e4-a52e-4f735466cecf'] ?? value.ELEMENT;
  }
  async wait(selector, timeout = 30_000) {
    const deadline = Date.now() + timeout;
    let last;
    while (Date.now() < deadline) {
      try { return await this.find(selector); } catch (error) { last = error; await new Promise((resolve) => setTimeout(resolve, 250)); }
    }
    let body = '';
    try { body = await this.bodyText(); } catch (error) { body = `body unavailable: ${error.message}`; }
    throw new Error(`element did not appear: ${selector}; last error: ${last}; body: ${body}`);
  }
  async click(element) { await this.request('POST', `/session/${this.sessionId}/element/${element}/click`, {}); }
  async value(element, value) {
    await this.request('POST', `/session/${this.sessionId}/element/${element}/clear`, {});
    await this.request('POST', `/session/${this.sessionId}/element/${element}/value`, { text: value, value: [...value] });
  }
  async invoke(command, input) {
    const result = await this.request('POST', `/session/${this.sessionId}/execute/async`, {
      script: `const done = arguments[arguments.length - 1];
        const invoke = window.__TAURI_INTERNALS__?.invoke;
        if (typeof invoke !== 'function') { done({ ok: false, error: 'Tauri invoke bridge unavailable' }); return; }
        invoke(arguments[0], { input: arguments[1] })
          .then((value) => done({ ok: true, value }))
          .catch((error) => done({ ok: false, error: String(error) }));`,
      args: [command, input],
    });
    if (!result?.ok) throw new Error(`Tauri command ${command} failed: ${result?.error ?? 'unknown error'}`);
    return result.value;
  }
  async text(element) { return this.request('GET', `/session/${this.sessionId}/element/${element}/text`); }
  async bodyText() { return this.text(await this.find('body')); }
  async screenshot(name) {
    const value = await this.request('GET', `/session/${this.sessionId}/screenshot`);
    await fs.writeFile(path.join(diagnostics, `${name}.png`), Buffer.from(value, 'base64'));
  }
  async waitForText(expected, timeout = 30_000) {
    const deadline = Date.now() + timeout;
    let actual = '';
    while (Date.now() < deadline) {
      try { actual = await this.bodyText(); if (actual.toLowerCase().includes(expected.toLowerCase())) return; } catch { /* retry while WebView settles */ }
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    throw new Error(`body did not contain ${JSON.stringify(expected)}; got ${JSON.stringify(actual)}`);
  }
}

let browser;
let phase = 'startup';
async function screenshot(name) { if (browser) await browser.screenshot(name); }
async function element(selector) { return browser.wait(selector); }
async function assertText(selector, expected) {
  const actual = await browser.text(await element(selector));
  if (!actual.toLowerCase().includes(expected.toLowerCase())) throw new Error(`${phase}: ${selector} did not contain ${JSON.stringify(expected)}; got ${JSON.stringify(actual)}`);
}

async function start() {
  browser = await new WebDriverSession().start();
  await element('[data-hank-frontend-mounted="true"]');
  await element('[data-hank-frontend-ready="true"]');
  await assertText('[aria-label^="Estado da aplicação"] .status', 'ready');
  await element('[aria-label="Gerenciamento de Projetos"]');
}
async function stop() {
  if (browser) { await browser.end(); browser = undefined; }
  await new Promise((resolve) => setTimeout(resolve, 1500));
}

try {
  await start();
  console.log(`DESKTOP E2E ARTIFACT: ${JSON.stringify(artifactIdentity)}`);
  phase = 'startup';
  await assertText('[aria-label="Gerenciamento de Projetos"]', 'Projetos');
  await browser.waitForText('Nenhum projeto encontrado');

  phase = 'create';
  await browser.click(await element('[aria-label="Abrir formulário de criação de projeto"]'));
  await browser.value(await element('#project-name-input'), projectName);
  await browser.value(await element('#project-owner-input'), owner);
  await browser.value(await element('#project-desc-input'), description);
  await browser.click(await element('button[type="submit"]'));
  await element(`[aria-label="Ver detalhes de ${projectName}"]`);
  await assertText('[aria-label="Gerenciamento de Projetos"]', projectName);
  await screenshot('01-created');

  phase = 'open';
  await browser.click(await element(`[aria-label="Ver detalhes de ${projectName}"]`));
  await element(`[aria-label="Detalhes do Projeto ${projectName}"]`);
  const detail = await browser.text(await element(`[aria-label="Detalhes do Projeto ${projectName}"]`));
  for (const expected of [projectName, owner, description, 'active']) if (!detail.toLowerCase().includes(expected.toLowerCase())) throw new Error(`open: missing ${expected}`);
  if (!/proj-[0-9a-f-]{36}/i.test(detail)) throw new Error('open: valid ProjectId was not displayed');
  await screenshot('02-opened');

  phase = 'agents';
  await browser.click(await element('[aria-label="Navegação principal"] button[aria-label="Agents"]'));
  await element('[aria-label="Gerenciamento de Agents"]');
  await browser.click(await element('[aria-label="Abrir formulário de criação de agent"]'));
  await browser.value(await element('#agent-create-name'), 'release-agent');
  await browser.value(await element('#agent-create-description'), 'Prepara releases com revisão humana.');
  await browser.click(await element('button[type="submit"]'));
  await browser.waitForText('release-agent');
  await browser.click(await element('[aria-label="Abrir conversas de release-agent"]'));
  await browser.waitForText('Nenhuma conversa iniciada para este agent.');
  await browser.click(await element('[aria-label="Abrir formulário de nova conversa"]'));
  await browser.value(await element('[id^="session-title-"]'), 'Release validation conversation');
  await browser.click(await element('.session-create-form button[type="submit"]'));
  await browser.waitForText('Release validation conversation');
  phase = 'session-open';
  await browser.click(await element('.session-open-button'));
  await element('[aria-label="Conversa Release validation conversation"]');
  await assertText('.session-workbench-notice', 'Envio de mensagens ainda não está integrado ao desktop.');
  await element('.session-workbench-composer textarea:disabled');
  await screenshot('04-session-opened');
  await browser.click(await element('[aria-label="Conversa Release validation conversation"] button'));
  await element('[aria-label="Conversas de release-agent"]');
  const projects = await browser.invoke('list_projects', {
    limit: 100,
    offset: 0,
    correlation_id: 'e2e-session-projects',
  });
  const project = projects.projects.find((candidate) => candidate.name === projectName);
  if (!project) throw new Error('sessions: created project was not returned by the real bridge');
  const agents = await browser.invoke('list_agents', {
    project_id: project.id,
    limit: 100,
    offset: 0,
    correlation_id: 'e2e-session-agents',
  });
  const agent = agents.agents.find((candidate) => candidate.name === 'release-agent');
  if (!agent) throw new Error('sessions: created agent was not returned by the real bridge');
  const sessions = await browser.invoke('list_sessions', {
    project_id: project.id,
    agent_id: agent.id,
    limit: 20,
    offset: 0,
    correlation_id: 'e2e-session-list',
  });
  if (sessions.total !== 1 || sessions.sessions[0]?.title !== 'Release validation conversation') {
    throw new Error(`sessions: UI-created session was not returned by the real bridge: ${JSON.stringify(sessions)}`);
  }
  if (sessions.sessions[0]?.status !== 'active') throw new Error('sessions: created session was not active');
  await screenshot('03-agents');
  await browser.click(await element('[aria-label="Conteúdo do projeto"] button[role="tab"]:first-child'));

  phase = 'update';
  await browser.click(await element('button.btn-edit'));
  await browser.value(await element('#edit-project-name'), updatedName);
  await browser.value(await element('#edit-project-desc'), updatedDescription);
  await browser.click(await element('button[type="submit"]'));
  await assertText('.project-detail-success', 'atualizado');
  await browser.waitForText(updatedName);
  await browser.click(await element('[aria-label="Voltar para a lista"]'));
  await element(`[aria-label="Ver detalhes de ${updatedName}"]`);

  phase = 'restart-1';
  await stop();
  await start();
  await browser.click(await element(`[aria-label="Ver detalhes de ${updatedName}"]`));
  await browser.waitForText(updatedDescription);
  await browser.waitForText(owner);
  await screenshot('04-after-restart-1');

  phase = 'archive';
  await browser.click(await element('button[aria-label="Arquivar este projeto"]'));
  await element('[role="dialog"]');
  await browser.value(await element('#archive-reason-input'), 'Desktop lifecycle E2E');
  await browser.click(await element('button.btn-danger'));
  await assertText('.project-detail-success', 'arquivado');
  await browser.waitForText('archived');
  await screenshot('05-archived');

  phase = 'restart-2';
  await stop();
  await start();
  await browser.click(await element(`[aria-label="Ver detalhes de ${updatedName}"]`));
  await browser.waitForText(updatedDescription);
  await browser.waitForText('archived');
  await screenshot('06-after-restart-2');
  console.log('DESKTOP E2E PROJECT LIFECYCLE: PASS');
} catch (error) {
  await screenshot(`failure-${phase}`).catch(() => {});
  console.error(`DESKTOP E2E PROJECT LIFECYCLE: FAIL at ${phase}`);
  console.error(error?.stack ?? error);
  process.exitCode = 1;
} finally {
  await stop().catch((error) => console.error('desktop shutdown failed:', error));
}
