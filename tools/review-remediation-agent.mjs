import { lstatSync, readFileSync, writeFileSync } from 'node:fs';

import { createGithubApi } from './review-remediation/github-api.mjs';
import {
  collectFinding,
  proposeFinding,
  publishValidated,
  renderDraftBody,
  validateProposal,
} from './review-remediation/orchestrator.mjs';

const COMMANDS = new Set(['collect', 'propose', 'validate', 'publish']);
const MAX_JSON_BYTES = 512 * 1024;

class CliError extends Error {
  constructor(code) {
    super(code);
    this.name = 'CliError';
    this.code = code;
  }
}

function error(code) {
  return new CliError(code);
}

function parseArgs(argv) {
  const command = argv[0];
  if (!COMMANDS.has(command)) throw error('CLI_USAGE');
  const args = { command };
  for (let index = 1; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith('--') || index + 1 >= argv.length || argv[index + 1].startsWith('--')) throw error('CLI_USAGE');
    const name = key.slice(2);
    if (!['event', 'input', 'output', 'repository', 'patch', 'patch-out', 'body-out', 'workspace', 'tests'].includes(name)) throw error('CLI_USAGE');
    if (args[name] !== undefined) throw error('CLI_USAGE');
    args[name] = argv[index + 1];
    index += 1;
  }
  return args;
}

function requireArg(args, name) {
  if (typeof args[name] !== 'string' || args[name].length === 0) throw error('CLI_USAGE');
  return args[name];
}

function readJson(path) {
  try {
    if (!lstatSync(path).isFile()) throw error('CLI_INPUT_INVALID');
    const text = readFileSync(path, 'utf8');
    if (Buffer.byteLength(text, 'utf8') > MAX_JSON_BYTES) throw error('CLI_INPUT_TOO_LARGE');
    return JSON.parse(text);
  } catch (caught) {
    if (caught instanceof CliError) throw caught;
    throw error('CLI_INPUT_INVALID');
  }
}

function readTests(path) {
  if (!path) return [];
  const tests = readJson(path);
  if (!Array.isArray(tests) || tests.some((value) => typeof value !== 'string')) throw error('CLI_TESTS_INVALID');
  return tests.slice(0, 20);
}

function writeJson(path, value) {
  requireArg({ output: path }, 'output');
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, { encoding: 'utf8', mode: 0o600 });
}

function repositoryFor(args) {
  return args.repository ?? process.env.GITHUB_REPOSITORY;
}

function githubApi(args) {
  return createGithubApi({
    token: process.env.GITHUB_TOKEN,
    repository: repositoryFor(args),
  });
}

async function run(args) {
  const output = requireArg(args, 'output');
  if (args.command === 'collect') {
    const eventPath = args.event ?? process.env.GITHUB_EVENT_PATH;
    const event = readJson(requireArg({ event: eventPath }, 'event'));
    const result = await collectFinding({
      event: { ...event, eventName: process.env.GITHUB_EVENT_NAME ?? event.eventName },
      repository: repositoryFor(args),
      api: githubApi(args),
    });
    writeJson(output, result);
    return;
  }

  const input = readJson(requireArg(args, 'input'));
  if (args.command === 'propose') {
    const result = await proposeFinding({
      collected: input,
      api: githubApi(args),
      apiKey: process.env.XIAOMI_MIMO_API_KEY,
      endpoint: process.env.MIMO_ENDPOINT,
      model: process.env.MIMO_MODEL,
    });
    if (result?.status === 'PROPOSED') {
      const patchOutput = requireArg(args, 'patch-out');
      writeFileSync(patchOutput, result.patch, { encoding: 'utf8', mode: 0o600 });
      const { patch: _patch, ...descriptor } = result;
      writeJson(output, descriptor);
    } else {
      writeJson(output, result);
    }
    return;
  }

  const patch = requireArg(args, 'patch');
  const workspace = requireArg(args, 'workspace');
  if (args.command === 'validate') {
    const result = await validateProposal({
      proposed: input,
      patchFile: patch,
      workspace,
      tests: readTests(args.tests),
    });
    writeJson(output, result);
    return;
  }

  const result = await publishValidated({
    validated: input,
    finding: input.finding,
    patchFile: patch,
    workspace,
    cycle: input.cycle,
  });
  if (result?.status === 'PUBLISH_READY' && args['body-out']) writeFileSync(requireArg(args, 'body-out'), renderDraftBody(result), { encoding: 'utf8', mode: 0o600 });
  writeJson(output, result);
}

try {
  await run(parseArgs(process.argv.slice(2)));
} catch (caught) {
  const code = caught?.code && /^[A-Z0-9_]+$/.test(caught.code) ? caught.code : 'CLI_FAILED';
  process.stderr.write(`review-remediation: ${code}\n`);
  process.exitCode = 1;
}
