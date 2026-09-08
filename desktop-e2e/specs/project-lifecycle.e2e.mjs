import { createHash, createPrivateKey, sign } from 'node:crypto';
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
  nativeBuild: null,
};
async function writeArtifactIdentity() {
  await fs.writeFile(path.join(diagnostics, 'artifact-identity.json'), JSON.stringify(artifactIdentity, null, 2));
}
await writeArtifactIdentity();

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
        invoke(arguments[0], arguments[1] === undefined ? undefined : { input: arguments[1] })
          .then((value) => done({ ok: true, value }))
          .catch((error) => done({ ok: false, error: String(error) }));`,
      args: input === undefined ? [command] : [command, input],
    });
    if (!result?.ok) throw new Error(`Tauri command ${command} failed: ${result?.error ?? 'unknown error'}`);
    return result.value;
  }
  async invokeRaw(command, args) {
    const result = await this.request('POST', `/session/${this.sessionId}/execute/async`, {
      script: `const done = arguments[arguments.length - 1];
        const invoke = window.__TAURI_INTERNALS__?.invoke;
        if (typeof invoke !== 'function') { done({ ok: false, error: 'Tauri invoke bridge unavailable' }); return; }
        invoke(arguments[0], arguments[1])
          .then((value) => done({ ok: true, value }))
          .catch((error) => done({ ok: false, detail: typeof error === 'string' ? error : JSON.stringify(error) }));`,
      args: [command, args],
    });
    if (!result?.ok) throw new Error(`Tauri command ${command} failed: ${result?.detail ?? 'unknown error'}`);
    return result.value;
  }
  async text(element) { return this.request('GET', `/session/${this.sessionId}/element/${element}/text`); }
  async bodyText() { return this.text(await this.find('body')); }
  async screenshot(name) {
    const value = await this.request('GET', `/session/${this.sessionId}/screenshot`);
    const base = path.resolve(diagnostics);
    const target = path.resolve(base, `${name}.png`);
    const relative = path.relative(base, target);
    if (relative.startsWith('..') || path.isAbsolute(relative)) {
      throw new Error('Invalid file path');
    }
    await fs.writeFile(target, Buffer.from(value, 'base64'));
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

function updaterCanonicalPayload(attestation) {
  return Buffer.from(JSON.stringify({
    schemaVersion: attestation.schemaVersion,
    artifact: attestation.artifact,
    identity: attestation.identity,
    signer: attestation.signer,
    update: attestation.update,
  }));
}

function updaterPlatform() {
  const arch = process.arch === 'x64' ? 'x86_64' : process.arch === 'arm64' ? 'aarch64' : process.arch;
  if (process.platform === 'win32') return { os: 'windows', arch };
  if (process.platform === 'darwin') return { os: 'macos', arch };
  return { os: 'linux', arch };
}

function signedUpdaterMetadata(version) {
  const privateKeyDerB64 = process.env.HANK_UPDATER_PRIVATE_KEY_DER_B64;
  if (!privateKeyDerB64) throw new Error('HANK_UPDATER_PRIVATE_KEY_DER_B64 is required for updater E2E');
  const key = createPrivateKey({ key: Buffer.from(privateKeyDerB64, 'base64'), format: 'der', type: 'pkcs8' });
  const platform = updaterPlatform();
  const expiresAt = 4102444800;
  const bytes = Buffer.from(`hank-updater-e2e-v${version}`);
  const attestation = {
    schemaVersion: 2,
    artifact: {
      name: `hank-update-v${version}.bin`,
      digest: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
      size: bytes.length,
    },
    identity: {
      repository: process.env.HANK_UPDATER_REPOSITORY ?? 'stoltembergg-png/hank',
      event: process.env.HANK_UPDATER_EVENT ?? 'workflow_dispatch',
      ref: process.env.HANK_UPDATER_REF ?? 'refs/heads/main',
      commit: process.env.HANK_UPDATER_COMMIT ?? 'a'.repeat(40),
      tree: process.env.HANK_UPDATER_TREE ?? 'b'.repeat(40),
      workflow: process.env.HANK_UPDATER_WORKFLOW ?? 'release.yml',
      policy: process.env.HANK_UPDATER_POLICY ?? 'updater-v1',
      channel: process.env.HANK_UPDATER_CHANNEL ?? 'stable',
      os: `${platform.os}-${platform.arch}`,
    },
    signer: { keyId: process.env.HANK_UPDATER_KEY_ID ?? 'e2e-fixture-v1' },
    update: { version, expiresAt, os: platform.os, arch: platform.arch },
    signature: { algorithm: 'ed25519', value: '' },
  };
  attestation.signature.value = sign(null, updaterCanonicalPayload(attestation), key).toString('base64');
  return {
    schema_version: 1,
    version,
    channel: process.env.HANK_UPDATER_CHANNEL ?? 'stable',
    os: platform.os,
    arch: platform.arch,
    size: bytes.length,
    expires_at: expiresAt,
    bytes: [...bytes],
    attestation,
    consent: true,
  };
}

async function protectedUpdaterMetadata(update) {
  const bundlePath = process.env.HANK_UPDATER_RELEASE_BUNDLE;
  const artifactPath = process.env.HANK_UPDATER_RELEASE_ARTIFACT;
  if (!bundlePath || !artifactPath) throw new Error('protected updater bundle and artifact are required');
  const bundle = JSON.parse(await fs.readFile(bundlePath, 'utf8'));
  const bytes = await fs.readFile(artifactPath);
  const entry = bundle.updates.find((candidate) => candidate.version === update);
  if (!entry) throw new Error(`protected updater version ${update} is missing from bundle`);
  if (entry.attestation.artifact.digest !== `sha256:${createHash('sha256').update(bytes).digest('hex')}`) {
    throw new Error('protected updater artifact digest mismatch');
  }
  return {
    schema_version: 1,
    version: entry.version,
    channel: entry.attestation.identity.channel,
    os: entry.os,
    arch: entry.arch,
    size: bytes.length,
    expires_at: entry.expiresAt,
    bytes: [...bytes],
    attestation: entry.attestation,
    consent: true,
  };
}

async function runUpdaterE2E() {
  if (process.env.HANK_UPDATER_E2E !== '1') return;
  const protectedRelease = Boolean(process.env.HANK_UPDATER_RELEASE_BUNDLE);
  const firstVersion = protectedRelease ? Number(process.env.HANK_UPDATER_CURRENT_VERSION) + 1 : 2;
  const secondVersion = protectedRelease ? firstVersion + 1 : 3;
  const first = await browser.invoke('stage_update', protectedRelease
    ? await protectedUpdaterMetadata(firstVersion)
    : signedUpdaterMetadata(firstVersion));
  if (first?.outcome !== 'staged' || first.version !== firstVersion) throw new Error(`updater: first version was not staged: ${JSON.stringify(first)}`);
  await browser.invoke('activate_update');
  const second = await browser.invoke('stage_update', protectedRelease
    ? await protectedUpdaterMetadata(secondVersion)
    : signedUpdaterMetadata(secondVersion));
  if (second?.outcome !== 'staged' || second.version !== secondVersion) throw new Error(`updater: second version was not staged: ${JSON.stringify(second)}`);
  await browser.invoke('activate_update');
  await browser.invoke('rollback_update');
  const recovery = await browser.invoke('recover_update');
  if (recovery !== 'clean') throw new Error(`updater: recovery after rollback was not clean: ${JSON.stringify(recovery)}`);
  const report = {
    status: 'PASS',
    evidenceScope: protectedRelease ? 'protected-release-signed-artifact' : 'native-synthetic-signed-fixture',
    platform: `${updaterPlatform().os}-${updaterPlatform().arch}`,
    stagedVersions: [firstVersion, secondVersion],
    activatedVersions: [firstVersion, secondVersion],
    rolledBackTo: firstVersion,
    recovery,
  };
  if (process.env.HANK_UPDATER_VERIFIED_DIGEST) report.artifactDigest = process.env.HANK_UPDATER_VERIFIED_DIGEST;
  await fs.writeFile(path.join(diagnostics, 'updater-rollback-report.json'), `${JSON.stringify(report, null, 2)}\n`);
  artifactIdentity.updater = report;
  await writeArtifactIdentity();
}

async function start() {
  browser = await new WebDriverSession().start();
  await element('[data-hank-frontend-mounted="true"]');
  await element('[data-hank-frontend-ready="true"]');
  const nativeBuild = await browser.invoke('build_identity');
  artifactIdentity.nativeBuild = nativeBuild;
  await writeArtifactIdentity();
  if (process.env.HANK_REQUIRE_ARTIFACT_PROVENANCE === '1') {
    const expectedCommit = process.env.HANK_EXPECTED_COMMIT_SHA;
    const expectedTree = process.env.HANK_EXPECTED_TREE_SHA;
    if (!expectedCommit || !expectedTree) throw new Error('artifact provenance expectation is incomplete');
    if (nativeBuild.commit_sha !== expectedCommit || nativeBuild.tree_sha !== expectedTree) {
      throw new Error(`artifact provenance mismatch: expected ${expectedCommit}/${expectedTree}, got ${JSON.stringify(nativeBuild)}`);
    }
    const expectedVersion = process.env.HANK_EXPECTED_NATIVE_VERSION;
    if (expectedVersion && nativeBuild.version !== expectedVersion) {
      throw new Error(`artifact version mismatch: expected ${expectedVersion}, got ${nativeBuild.version}`);
    }
  }
  await assertText('[aria-label^="Estado da aplicação"] .status', 'ready');
  await element('[aria-label="Gerenciamento de Projetos"]');
}
async function stop() {
  if (browser) { await browser.end(); browser = undefined; }
  await new Promise((resolve) => setTimeout(resolve, 1500));
}

try {
  await start();
  phase = 'updater';
  await runUpdaterE2E();
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
  const chatProjects = await browser.invoke('list_projects', {
    limit: 100,
    offset: 0,
    correlation_id: 'e2e-chat-provider-projects',
  });
  const chatProject = chatProjects.projects.find((candidate) => candidate.name === projectName);
  if (!chatProject) throw new Error('provider: project was not available before chat credential setup');
  phase = 'provider-chat-precondition';
  const chatOAuth = await browser.invoke('start_provider_oauth', {
    project_id: chatProject.id,
    provider_id: 'mock',
    account_id: 'account_mock',
  });
  const chatAuthorization = new URL(chatOAuth.authorization_url);
  const chatState = chatAuthorization.searchParams.get('state');
  if (!chatState) throw new Error(`provider: chat precondition authorization state is missing: ${chatOAuth.authorization_url}`);
  const chatConnection = await browser.invoke('complete_provider_oauth', {
    project_id: chatProject.id,
    callback_url: `hank://oauth/callback?flow=${chatOAuth.flow_id}&provider=mock&account=account_mock&state=${chatState}&code=fixture`,
  });
  if (chatConnection.state !== 'connected' || !chatConnection.account?.has_credential_ref) {
    throw new Error(`provider: chat precondition did not establish a credential: ${JSON.stringify(chatConnection)}`);
  }
  phase = 'chat';
  await element('[aria-label="Chat da sessão"]');
  const chatInput = await element('#chat-message');
  await browser.value(chatInput, 'release smoke');
  await browser.click(await element('[aria-label="Chat da sessão"] button[type="submit"]'));
  await browser.waitForText('mock response: release smoke');
  await assertText('.chat-status', 'Concluída');
  await screenshot('04-chat-completed');
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
  if (!/^trace-[0-9a-f-]{36}$/i.test(sessions.sessions[0]?.trace_id ?? '')) {
    throw new Error(`chat: session trace identity is missing or malformed: ${JSON.stringify(sessions.sessions[0])}`);
  }
  if (sessions.sessions[0]?.message_count !== 2) {
    throw new Error(`chat: persisted session message count was not updated: ${JSON.stringify(sessions.sessions[0])}`);
  }
  const chatMessages = await browser.invoke('list_chat_messages', {
    project_id: project.id,
    agent_id: agent.id,
    session_id: sessions.sessions[0].id,
    caller: { caller_id: 'desktop-webview', class: 'desktop' },
    limit: 100,
    offset: 0,
  });
  if (chatMessages.messages?.length !== 2 || chatMessages.messages[0]?.role !== 'user' || chatMessages.messages[1]?.role !== 'assistant') {
    throw new Error(`chat: persisted message history is invalid: ${JSON.stringify(chatMessages)}`);
  }
  if (!chatMessages.messages[1].text.includes('mock response: release smoke')) {
    throw new Error(`chat: assistant response was not persisted: ${JSON.stringify(chatMessages)}`);
  }
  const chatUsage = await browser.invoke('get_chat_usage', {
    project_id: project.id,
    agent_id: agent.id,
    session_id: sessions.sessions[0].id,
    caller: { caller_id: 'desktop-webview', class: 'desktop' },
  });
  if (chatUsage.usage?.source !== 'missing' || chatUsage.usage?.confidence !== 'unavailable' || chatUsage.usage?.missing_usage_count !== 1) {
    throw new Error(`chat: usage must remain explicitly unavailable when the stream omits provider usage: ${JSON.stringify(chatUsage)}`);
  }

  phase = 'provider-oauth-negative';
  const providerAccounts = await browser.invoke('list_provider_accounts', {
    project_id: project.id,
  });
  const mockAccount = providerAccounts.find((account) => account.provider_id === 'mock' && account.account_id === 'account_mock');
  if (!mockAccount || mockAccount.state !== 'connected' || !mockAccount.has_credential_ref) {
    throw new Error(`provider: shared credential state was not visible after chat setup: ${JSON.stringify(providerAccounts)}`);
  }
  const initialRevoked = await browser.invoke('disconnect_provider_account', {
    project_id: project.id,
    provider_id: 'mock',
    account_id: 'account_mock',
  });
  if (initialRevoked.state !== 'revoked' || initialRevoked.has_credential_ref) {
    throw new Error(`provider: initial disconnect did not revoke the shared credential: ${JSON.stringify(initialRevoked)}`);
  }
  const oauth = await browser.invoke('start_provider_oauth', {
    project_id: project.id,
    provider_id: 'mock',
    account_id: 'account_mock',
  });
  if (!/^flow_\d+$/.test(oauth.flow_id) || oauth.state !== 'pending') {
    throw new Error(`provider: OAuth flow did not start in pending state: ${JSON.stringify(oauth)}`);
  }
  const pending = await browser.invoke('get_provider_oauth_status', {
    project_id: project.id,
    flow_id: oauth.flow_id,
  });
  if (pending.state !== 'pending') throw new Error(`provider: fresh OAuth flow was not pending: ${JSON.stringify(pending)}`);
  let callbackRejected = false;
  try {
    await browser.invoke('complete_provider_oauth', {
      project_id: project.id,
      callback_url: `hank://oauth/callback?flow=${oauth.flow_id}&provider=mock&account=account_mock&state=state_invalid&code=fixture`,
    });
  } catch (error) {
    callbackRejected = Boolean(error);
  }
  if (!callbackRejected) throw new Error('provider: callback with an invalid state was accepted');
  const invalid = await browser.invoke('get_provider_oauth_status', {
    project_id: project.id,
    flow_id: oauth.flow_id,
  });
  if (invalid.state !== 'invalid' || invalid.error_code !== 'state_mismatch') {
    throw new Error(`provider: invalid callback did not produce a stable redacted status: ${JSON.stringify(invalid)}`);
  }
  const reset = await browser.invoke('disconnect_provider_account', {
    project_id: project.id,
    provider_id: 'mock',
    account_id: 'account_mock',
  });
  if (reset.state !== 'revoked' || reset.has_credential_ref) {
    throw new Error(`provider: disconnect did not clear fixture account state: ${JSON.stringify(reset)}`);
  }
  const validOAuth = await browser.invoke('start_provider_oauth', {
    project_id: project.id,
    provider_id: 'mock',
    account_id: 'account_mock',
  });
  if (!validOAuth.authorization_url?.startsWith('hank://oauth/authorize?')) {
    throw new Error(`provider: OAuth start did not return a bounded authorization URL: ${JSON.stringify(validOAuth)}`);
  }
  const authorization = new URL(validOAuth.authorization_url);
  const state = authorization.searchParams.get('state');
  if (!state || authorization.searchParams.get('flow') !== validOAuth.flow_id) {
    throw new Error(`provider: authorization URL omitted flow/state binding: ${validOAuth.authorization_url}`);
  }
  const connected = await browser.invoke('complete_provider_oauth', {
    project_id: project.id,
    callback_url: `hank://oauth/callback?flow=${validOAuth.flow_id}&provider=mock&account=account_mock&state=${state}&code=fixture`,
  });
  if (connected.state !== 'connected' || connected.account?.project_id !== project.id || !connected.account?.has_credential_ref) {
    throw new Error(`provider: valid callback did not connect the scoped account: ${JSON.stringify(connected)}`);
  }
  let replayRejected = false;
  try {
    await browser.invoke('complete_provider_oauth', {
      project_id: project.id,
      callback_url: `hank://oauth/callback?flow=${validOAuth.flow_id}&provider=mock&account=account_mock&state=${state}&code=fixture`,
    });
  } catch (error) {
    replayRejected = Boolean(error);
  }
  if (!replayRejected) throw new Error('provider: replayed OAuth callback was accepted');
  const afterReplay = await browser.invoke('get_provider_oauth_status', {
    project_id: project.id,
    flow_id: validOAuth.flow_id,
  });
  if (afterReplay.state !== 'connected') {
    throw new Error(`provider: replay changed a connected flow state: ${JSON.stringify(afterReplay)}`);
  }
  const revoked = await browser.invoke('disconnect_provider_account', {
    project_id: project.id,
    provider_id: 'mock',
    account_id: 'account_mock',
  });
  if (revoked.state !== 'revoked' || revoked.has_credential_ref) {
    throw new Error(`provider: disconnect did not clear fixture account state: ${JSON.stringify(revoked)}`);
  }
  let chatBlockedAfterDisconnect = false;
  let chatBlockError = null;
  try {
    await browser.invokeRaw('send_chat_command', {
      command: {
        schema_version: 1,
        command_id: 'e2e-revoked-chat-command',
        stream_id: 'e2e-revoked-chat-stream',
        caller: { caller_id: 'desktop-webview', class: 'desktop' },
        project_id: project.id,
        agent_id: agent.id,
        session_id: sessions.sessions[0].id,
        text: 'revoked credential must block chat',
        generation: 2,
        cancellation_id: 'e2e-revoked-chat-cancel',
      },
    });
  } catch (error) {
    chatBlockError = String(error);
    chatBlockedAfterDisconnect = chatBlockError.includes('send_chat_command failed');
  }
  if (!chatBlockedAfterDisconnect) throw new Error(`provider: revoked credential still allowed a chat command; result=${chatBlockError ?? 'command succeeded'}`);

  phase = 'provider-settings-ui';
  await browser.click(await element('[aria-label="Configurações"]'));
  await element('.provider-settings-page');
  await assertText('.provider-settings-header h1', 'Configurações de providers');
  await browser.waitForText('revoked');
  await browser.click(await element('.provider-settings-header button'));
  await element(`[aria-label="Detalhes do Projeto ${projectName}"]`);

  await screenshot('03-agents');
  await browser.click(await element('[aria-label="Conteúdo do projeto"] button[role="tab"]:first-child'));

  phase = 'workflow-save';
  await browser.click(await element('[aria-label="Workflows"]'));
  await element('[aria-label="Workflows do projeto"]');
  await browser.click(await element('[aria-label="Adicionar nó Agent"]'));
  await browser.click(await element('.workflow-save-button'));
  await browser.waitForText('Workflow salvo na versão 1.');
  await browser.waitForText('Agent 1');
  const staleWorkflowValidation = await browser.invoke('validate_workflow', {
    project_id: project.id,
    workflow_id: 'wf-00000000-0000-4000-8000-000000000001',
    expected_version: 0,
    draft: {
      project_id: project.id,
      workflow_id: 'wf-00000000-0000-4000-8000-000000000001',
      nodes: [{ id: 'agent-1', kind: 'agent', label: 'Agent 1' }],
      edges: [],
    },
  });
  if (staleWorkflowValidation.valid || staleWorkflowValidation.reason !== 'stale_version') {
    throw new Error(`workflow: stale validation was accepted: ${JSON.stringify(staleWorkflowValidation)}`);
  }
  await screenshot('04-workflow-saved');
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
  await screenshot('05-after-restart-1');

  phase = 'workflow-reload';
  await browser.click(await element('[aria-label="Workflows"]'));
  await element('[aria-label="Workflows do projeto"]');
  await browser.waitForText('Agent 1');
  await assertText('.workflow-surface-notice', 'A persistência está disponível');
  await browser.click(await element('[aria-label="Conteúdo do projeto"] button[role="tab"]:first-child'));

  phase = 'archive';
  await browser.click(await element('button[aria-label="Arquivar este projeto"]'));
  await element('[role="dialog"]');
  await browser.value(await element('#archive-reason-input'), 'Desktop lifecycle E2E');
  await browser.click(await element('button.btn-danger'));
  await assertText('.project-detail-success', 'arquivado');
  await browser.waitForText('archived');
  await screenshot('06-archived');

  phase = 'restart-2';
  await stop();
  await start();
  await browser.click(await element(`[aria-label="Ver detalhes de ${updatedName}"]`));
  await browser.waitForText(updatedDescription);
  await browser.waitForText('archived');
  await screenshot('07-after-restart-2');
  console.log('DESKTOP E2E PROJECT LIFECYCLE: PASS');
} catch (error) {
  await screenshot(`failure-${phase}`).catch(() => {});
  console.error(`DESKTOP E2E PROJECT LIFECYCLE: FAIL at ${phase}`);
  console.error(error?.stack ?? error);
  process.exitCode = 1;
} finally {
  await stop().catch((error) => console.error('desktop shutdown failed:', error));
}
