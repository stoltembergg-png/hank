import { closeSync, constants, fstatSync, lstatSync, openSync, readSync, realpathSync, writeFileSync } from 'node:fs';
import { isAbsolute, relative, resolve } from 'node:path';

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
    if (!['event', 'input', 'proposal', 'output', 'repository', 'patch', 'patch-out', 'body-out', 'workspace', 'tests'].includes(name)) throw error('CLI_USAGE');
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

function pathWithin(root, candidate) {
  const relativePath = relative(root, candidate);
  return relativePath === '' || (!relativePath.startsWith('..') && !isAbsolute(relativePath));
}

function hasParentSegment(value) {
  return value.replaceAll('\\', '/').split('/').includes('..');
}

function resolveCliPath(path, { allowEventPath = false } = {}) {
  if (typeof path !== 'string' || path.length === 0 || path.includes('\u0000')) throw error('CLI_INPUT_INVALID');
  const candidate = resolve(path);
  const workspace = resolve(process.env.GITHUB_WORKSPACE ?? process.cwd());
  const eventPath = process.env.GITHUB_EVENT_PATH ? resolve(process.env.GITHUB_EVENT_PATH) : undefined;
  const isEventPath = allowEventPath && eventPath === candidate;
  if (!pathWithin(workspace, candidate) && !isEventPath) throw error('CLI_INPUT_INVALID');
  try {
    const workspaceRoot = realpathSync(workspace);
    const canonical = realpathSync(candidate);
    if (!isEventPath && !pathWithin(workspaceRoot, canonical)) throw error('CLI_INPUT_INVALID');
    if (lstatSync(candidate).isSymbolicLink()) throw error('CLI_INPUT_INVALID');
  } catch (caught) {
    if (caught instanceof CliError) throw caught;
    throw error('CLI_INPUT_INVALID');
  }
  return candidate;
}

function resolveCliReadTarget(path, options) {
  const inputPath = resolveCliPath(path, options);
  const workspace = resolve(process.env.GITHUB_WORKSPACE ?? process.cwd());
  const eventPath = process.env.GITHUB_EVENT_PATH ? resolve(process.env.GITHUB_EVENT_PATH) : undefined;
  const isEventPath = options?.allowEventPath === true && eventPath === inputPath;
  const root = isEventPath ? resolve(inputPath, '..') : realpathSync(workspace);
  const canonical = realpathSync(inputPath);
  const relativePath = relative(root, canonical);
  if (!relativePath || relativePath.startsWith('..') || isAbsolute(relativePath)) throw error('CLI_INPUT_INVALID');
  return { root, relativePath };
}

function resolveCliOutputPath(path) {
  if (typeof path !== 'string' || path.length === 0 || path.includes('\u0000')) throw error('CLI_INPUT_INVALID');
  const candidate = resolve(path);
  const workspace = resolve(process.env.GITHUB_WORKSPACE ?? process.cwd());
  if (!pathWithin(workspace, candidate)) throw error('CLI_INPUT_INVALID');
  try {
    const workspaceRoot = realpathSync(workspace);
    const parent = realpathSync(resolve(candidate, '..'));
    if (!pathWithin(workspaceRoot, parent)) throw error('CLI_INPUT_INVALID');
    if (lstatSync(candidate).isSymbolicLink()) throw error('CLI_INPUT_INVALID');
  } catch (caught) {
    if (caught instanceof CliError) throw caught;
    if (caught?.code !== 'ENOENT') throw error('CLI_INPUT_INVALID');
  }
  return candidate;
}

function readBoundedFile({ root, relativePath }) {
  if (typeof root !== 'string' || typeof relativePath !== 'string'
    || relativePath.length === 0 || relativePath.includes('\u0000')
    || hasParentSegment(relativePath) || isAbsolute(relativePath)) {
    throw error('CLI_INPUT_INVALID');
  }
  const filePath = resolve(root, relativePath);
  const relativeCheck = relative(root, filePath);
  if (relativeCheck.startsWith('..') || isAbsolute(relativeCheck)) throw error('CLI_INPUT_INVALID');
  let descriptor;
  try {
    const noFollow = constants.O_NOFOLLOW ?? 0;
    descriptor = openSync(filePath, constants.O_RDONLY | noFollow);
    const stat = fstatSync(descriptor);
    if (!stat.isFile()) throw error('CLI_INPUT_INVALID');
    if (stat.size > MAX_JSON_BYTES) throw error('CLI_INPUT_TOO_LARGE');
    const content = Buffer.alloc(MAX_JSON_BYTES + 1);
    let bytesRead = 0;
    while (bytesRead < content.byteLength) {
      const result = readSync(descriptor, content, bytesRead, content.byteLength - bytesRead, null);
      if (result === 0) break;
      bytesRead += result;
    }
    if (bytesRead > MAX_JSON_BYTES || fstatSync(descriptor).size > MAX_JSON_BYTES) throw error('CLI_INPUT_TOO_LARGE');
    return content.subarray(0, bytesRead).toString('utf8');
  } catch (caught) {
    if (caught instanceof CliError) throw caught;
    throw error('CLI_INPUT_INVALID');
  } finally {
    if (descriptor !== undefined) {
      try {
        closeSync(descriptor);
      } catch {
        // Preserve the original CLI validation result.
      }
    }
  }
}

function readJson(path, options) {
  try {
    const target = resolveCliReadTarget(path, options);
    const inputPath = resolve(target.root, target.relativePath);
    if (!lstatSync(inputPath).isFile()) throw error('CLI_INPUT_INVALID');
    return JSON.parse(readBoundedFile(target));
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
  const outputPath = resolveCliOutputPath(path);
  writeFileSync(outputPath, `${JSON.stringify(value, null, 2)}\n`, { encoding: 'utf8', mode: 0o600 });
}

function writeText(path, value) {
  writeFileSync(resolveCliOutputPath(requireArg({ output: path }, 'output')), value, { encoding: 'utf8', mode: 0o600 });
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
    const event = readJson(requireArg({ event: eventPath }, 'event'), { allowEventPath: true });
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
      writeText(patchOutput, result.patch);
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

  const proposal = readJson(requireArg(args, 'proposal'));
  const result = await publishValidated({
    validated: input,
    proposal,
    finding: proposal.finding,
    patchFile: patch,
    workspace,
    api: githubApi(args),
    cycle: input.cycle,
  });
  if (result?.status === 'PUBLISH_READY' && args['body-out']) writeText(args['body-out'], renderDraftBody(result));
  writeJson(output, result);
}

try {
  await run(parseArgs(process.argv.slice(2)));
} catch (caught) {
  const code = caught?.code && /^[A-Z0-9_]+$/.test(caught.code) ? caught.code : 'CLI_FAILED';
  process.stderr.write(`review-remediation: ${code}\n`);
  process.exitCode = 1;
}
