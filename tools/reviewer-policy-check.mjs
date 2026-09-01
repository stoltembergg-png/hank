import { readFileSync } from 'node:fs';

const configPath = process.argv[2] ?? '.coderabbit.yaml';

function parseYaml(source) {
  const lines = source.split(/\r?\n/);
  const root = {};
  const stack = [{ indent: -1, node: root, key: null }];
  let inBlockScalar = false;
  let blockScalarIndent = 0;

  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i];
    const trimmed = rawLine.trim();
    const indent = rawLine.length - rawLine.trimStart().length;

    // Skip empty lines and comments
    if (trimmed === '' || trimmed.startsWith('#')) {
      if (inBlockScalar && indent > blockScalarIndent) {
        const top = stack[stack.length - 1];
        if (top.key) top.node[top.key] += '\n' + rawLine.slice(blockScalarIndent);
      }
      continue;
    }

    // Handle block scalar continuation
    if (inBlockScalar) {
      if (indent > blockScalarIndent) {
        const top = stack[stack.length - 1];
        if (top.key) top.node[top.key] += '\n' + rawLine.slice(blockScalarIndent);
        continue;
      } else {
        inBlockScalar = false;
      }
    }

    const colonIndex = trimmed.indexOf(':');
    if (colonIndex === -1) continue;

    const key = trimmed.slice(0, colonIndex).trim();
    const value = trimmed.slice(colonIndex + 1).trim();

    // Pop stack to find correct parent
    while (stack.length > 1 && indent <= stack[stack.length - 1].indent) {
      stack.pop();
    }

    const parent = stack[stack.length - 1];

    // Check if this is a mapping (empty value followed by indented content)
    // or a block scalar indicator
    if (value === '' || value === '|') {
      // Look ahead to see if next non-empty, non-comment line has greater indent
      let isBlockScalar = false;
      let isMapping = false;

      for (let j = i + 1; j < lines.length; j++) {
        const nextRaw = lines[j];
        const nextTrimmed = nextRaw.trim();
        const nextIndent = nextRaw.length - nextRaw.trimStart().length;

        if (nextTrimmed === '' || nextTrimmed.startsWith('#')) continue;

        if (nextIndent > indent) {
          // Could be mapping or block scalar
          // If it contains a colon, it's a mapping
          if (nextTrimmed.includes(':')) {
            isMapping = true;
          } else if (value === '|') {
            isBlockScalar = true;
          }
        }
        break;
      }

      if (isBlockScalar) {
        inBlockScalar = true;
        blockScalarIndent = indent;
        parent.node[key] = '';
        stack.push({ indent, node: parent.node, key });
      } else {
        // It's a nested mapping
        const newNode = {};
        parent.node[key] = newNode;
        stack.push({ indent, node: newNode, key: null });
      }
    } else {
      // Simple scalar value
      let parsedValue = value;
      if (value === 'true' || value === 'false') {
        parsedValue = value === 'true';
      } else if (/^-?\d+$/.test(value)) {
        parsedValue = parseInt(value, 10);
      } else if (/^-?\d*\.\d+$/.test(value)) {
        parsedValue = parseFloat(value);
      } else if ((value.startsWith('"') && value.endsWith('"')) ||
                 (value.startsWith("'") && value.endsWith("'"))) {
        parsedValue = value.slice(1, -1);
      }
      parent.node[key] = parsedValue;
    }
  }

  return root;
}

function getAtPath(obj, path) {
  const parts = path.split('.');
  let current = obj;
  for (const part of parts) {
    if (current === null || current === undefined || typeof current !== 'object') {
      return undefined;
    }
    current = current[part];
  }
  return current;
}

function validate(source) {
  const config = parseYaml(source);

  const errors = [];

  const reviews = getAtPath(config, 'reviews');
  if (!reviews) {
    errors.push('reviews block is required');
    return errors;
  }

  const requestChanges = getAtPath(config, 'reviews.request_changes_workflow');
  if (requestChanges !== false) {
    errors.push('reviews.request_changes_workflow must be false');
  }

  const failCommit = getAtPath(config, 'reviews.fail_commit_status');
  if (failCommit !== true) {
    errors.push('reviews.fail_commit_status must be true');
  }

  const autoReviewEnabled = getAtPath(config, 'reviews.auto_review.enabled');
  if (autoReviewEnabled !== true) {
    errors.push('reviews.auto_review.enabled must be true');
  }

  return errors;
}

let source;
try {
  source = readFileSync(configPath, 'utf8');
} catch (error) {
  process.stderr.write(`cannot read reviewer configuration ${configPath}: ${error.message}\n`);
  process.exitCode = 1;
}

if (source !== undefined) {
  const errors = validate(source);
  if (errors.length > 0) {
    process.stderr.write(`${errors.join('\n')}\n`);
    process.exitCode = 1;
  }
}