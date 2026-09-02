import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../', import.meta.url);
const read = (path) => readFileSync(new URL(path, root), 'utf8');

const rustWorkflow = read('.github/workflows/build-rust.yml');
const frontendWorkflow = read('.github/workflows/build-frontend.yml');
const tauriWorkflow = read('.github/workflows/build-tauri.yml');
const codeqlWorkflow = read('.github/workflows/codeql.yml');
const qualityIntegrityWorkflow = read('.github/workflows/quality-integrity.yml');

function yamlMapping(line, index) {
  const match = /^(\s*)([A-Za-z_][A-Za-z0-9_-]*):(?:\s*(.*))?$/.exec(line);
  if (!match) return undefined;

  return {
    index,
    indentation: match[1].length,
    name: match[2],
    value: (match[3] ?? '').trim(),
  };
}

function directMappings(lines, start, end, parentIndentation) {
  const entries = [];
  for (let index = start; index < end; index += 1) {
    const entry = yamlMapping(lines[index], index);
    if (entry && entry.indentation > parentIndentation) entries.push(entry);
  }
  if (entries.length === 0) return [];

  const directIndentation = Math.min(...entries.map((entry) => entry.indentation));
  return entries.filter((entry) => entry.indentation === directIndentation);
}

function blockFromEntry(lines, entry, end) {
  let blockEnd = end;
  for (let index = entry.index + 1; index < end; index += 1) {
    const line = lines[index];
    if (line.trim() === '' || line.trimStart().startsWith('#')) continue;
    if (line.length - line.trimStart().length <= entry.indentation) {
      blockEnd = index;
      break;
    }
  }

  return { start: entry.index + 1, end: blockEnd, indentation: entry.indentation };
}

function findBlock(lines, start, end, parentIndentation, name) {
  const matches = directMappings(lines, start, end, parentIndentation)
    .filter((entry) => entry.name === name && entry.value === '');
  if (matches.length !== 1) return undefined;
  return blockFromEntry(lines, matches[0], end);
}

function runBody(lines, runEntry, end) {
  if (!/^[|>][+-]?(?:\s+#.*)?$/.test(runEntry.value)) return [runEntry.value];

  const body = [];
  for (let index = runEntry.index + 1; index < end; index += 1) {
    const line = lines[index];
    if (line.trim() !== '' && line.length - line.trimStart().length <= runEntry.indentation) break;
    body.push(line.trim());
  }

  const executable = [];
  let heredocDelimiter;
  for (const line of body) {
    if (heredocDelimiter) {
      if (line === heredocDelimiter) heredocDelimiter = undefined;
      continue;
    }

    const heredoc = /<<-?\s*(?:(['"])(.*?)\1|([^\s;|&]+))/.exec(line);
    executable.push(line);
    if (heredoc) heredocDelimiter = heredoc[2] ?? heredoc[3];
  }
  return executable;
}

function executableRunCommands(workflow, jobName) {
  const lines = workflow.split(/\r?\n/);
  const jobs = findBlock(lines, 0, lines.length, -1, 'jobs');
  if (!jobs) return [];

  const jobEntries = directMappings(lines, jobs.start, jobs.end, jobs.indentation)
    .filter((entry) => entry.value === '')
    .filter((entry) => jobName === undefined || entry.name === jobName);

  return jobEntries.flatMap((jobEntry) => {
    const job = blockFromEntry(lines, jobEntry, jobs.end);
    const steps = findBlock(lines, job.start, job.end, job.indentation, 'steps');
    if (!steps) return [];

    const stepIndentation = steps.indentation + 2;
    const runIndentation = stepIndentation + 2;
    let inStep = false;
    const commands = [];

    for (let index = steps.start; index < steps.end; index += 1) {
      const line = lines[index];
      const indentation = line.length - line.trimStart().length;
      if (indentation === stepIndentation && /^\s*-\s/.test(line)) {
        inStep = true;
        const inlineRun = new RegExp(`^\\s*-\\s+run:\\s*(.*)$`).exec(line);
        if (inlineRun) {
          commands.push(...runBody(lines, {
            index,
            indentation: runIndentation,
            value: inlineRun[1].trim(),
          }, steps.end));
        }
        continue;
      }
      if (!inStep) continue;

      const entry = yamlMapping(line, index);
      if (entry?.indentation === runIndentation && entry.name === 'run') {
        commands.push(...runBody(lines, entry, steps.end));
      }
    }

    return commands;
  });
}

function assertCommand(workflow, command, file, jobName) {
  const found = executableRunCommands(workflow, jobName)
    .some((run) => run === command);
  assert.ok(found, `${file}: missing ${command}`);
}

test('Rust workflow has fail-closed quality commands', () => {
  assertCommand(rustWorkflow, 'cargo fmt --all -- --check', 'build-rust.yml');
  assertCommand(rustWorkflow, 'cargo test --workspace --locked', 'build-rust.yml');
  assertCommand(rustWorkflow, 'cargo clippy --workspace --all-targets --locked -- -D warnings', 'build-rust.yml');
  assert.doesNotMatch(rustWorkflow, /continue-on-error\s*:\s*true/);
});

test('Frontend workflow has explicit lint, typecheck, test, and build gates', () => {
  assertCommand(frontendWorkflow, 'npm run lint', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm run typecheck', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm run test', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm run build', 'build-frontend.yml');
  assertCommand(frontendWorkflow, 'npm audit --audit-level=high', 'build-frontend.yml');
  assert.doesNotMatch(frontendWorkflow, /continue-on-error\s*:\s*true/);
});

test('Tauri workflow retains native check, format, and acceptance gates', () => {
  assertCommand(tauriWorkflow, 'cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --locked', 'build-tauri.yml');
  assertCommand(tauriWorkflow, 'cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check', 'build-tauri.yml');
  assertCommand(tauriWorkflow, 'cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked', 'build-tauri.yml');
  assert.doesNotMatch(tauriWorkflow, /continue-on-error\s*:\s*true/);
});

test('CodeQL workflow is pinned, scoped, and fail-closed', () => {
  assert.match(codeqlWorkflow, /security-events:\s*write/);
  assert.match(codeqlWorkflow, /languages:\s*\$\{\{ matrix\.language \}\}/);
  assert.match(codeqlWorkflow, /github\/codeql-action\/init@[0-9a-f]{40}/);
  assert.match(codeqlWorkflow, /github\/codeql-action\/autobuild@[0-9a-f]{40}/);
  assert.match(codeqlWorkflow, /github\/codeql-action\/analyze@[0-9a-f]{40}/);
  assert.match(codeqlWorkflow, /matrix\.language == 'rust'/);
  assert.doesNotMatch(codeqlWorkflow, /continue-on-error\s*:\s*true/);
});

test('quality integrity validates the hardened automated reviewer policy', () => {
  assertCommand(
    qualityIntegrityWorkflow,
    'node --test tools/reviewer-policy-check.spec.mjs',
    'quality-integrity.yml',
    'integrity',
  );
});

test('workflow contract ignores comments, echo, and another job', () => {
  const decoyWorkflow = `# node --test tools/reviewer-policy-check.spec.mjs
jobs:
  decoy:
    steps:
      - run: echo "node --test tools/reviewer-policy-check.spec.mjs"
  integrity:
    steps:
      - name: Comment only
        run: |
          # node --test tools/reviewer-policy-check.spec.mjs
`;

  assert.throws(
    () => assertCommand(
      decoyWorkflow,
      'node --test tools/reviewer-policy-check.spec.mjs',
      'fixture.yml',
      'integrity',
    ),
    /fixture\.yml: missing node --test tools\/reviewer-policy-check\.spec\.mjs/,
  );
});

test('workflow contract recognizes inline multiline run steps', () => {
  const multilineWorkflow = `jobs:
  integrity:
    steps:
      - run: |
          node --test tools/reviewer-policy-check.spec.mjs
      - run: >
          node --test tools/reviewer-policy-check.spec.mjs
`;

  assertCommand(
    multilineWorkflow,
    'node --test tools/reviewer-policy-check.spec.mjs',
    'fixture.yml',
    'integrity',
  );
});

test('workflow contract does not treat heredoc content as an executable command', () => {
  const heredocWorkflow = `jobs:
  integrity:
    steps:
      - name: Heredoc content
        run: |
          cat <<'EOF'
          node --test tools/reviewer-policy-check.spec.mjs
          EOF
`;

  assert.throws(
    () => assertCommand(
      heredocWorkflow,
      'node --test tools/reviewer-policy-check.spec.mjs',
      'fixture.yml',
      'integrity',
    ),
    /fixture\.yml: missing node --test tools\/reviewer-policy-check\.spec\.mjs/,
  );
});

test('workflow contract rejects commands that can mask failures', () => {
  const failOpenWorkflow = `jobs:
  integrity:
    steps:
      - name: Fail open
        run: node --test tools/reviewer-policy-check.spec.mjs || true
`;

  assert.throws(
    () => assertCommand(
      failOpenWorkflow,
      'node --test tools/reviewer-policy-check.spec.mjs',
      'fixture.yml',
      'integrity',
    ),
    /fixture\.yml: missing node --test tools\/reviewer-policy-check\.spec\.mjs/,
  );
});
