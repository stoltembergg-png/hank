import { readFileSync } from 'node:fs';

const configPath = process.argv[2] ?? '.coderabbit.yaml';

function indentation(line) {
  return line.length - line.trimStart().length;
}

function isBlockScalar(value) {
  return /^(?:[|>](?:[1-9])?[-+]?|[|>][-+]?[1-9])$/.test(value);
}

function mapping(line, index) {
  const match = /^(\s*)([A-Za-z_][A-Za-z0-9_-]*):(?:\s*(.*))?$/.exec(line);
  if (!match) return undefined;
  return {
    index,
    indentation: match[1].length,
    name: match[2],
    value: (match[3] ?? '').replace(/(?:^|\s+)#.*$/, '').trim(),
  };
}

function structuralMappings(lines) {
  const entries = [];
  const sequenceIndentations = [];
  let scalar;

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const lineIndentation = indentation(line);

    if (scalar) {
      if (line.trim() === '' || line.trimStart().startsWith('#')) continue;
      if (lineIndentation > scalar.indentation) continue;
      scalar = undefined;
    }

    while (sequenceIndentations.at(-1) >= lineIndentation) sequenceIndentations.pop();
    if (/^\s*-\s/.test(line)) {
      sequenceIndentations.push(lineIndentation);
      continue;
    }

    const entry = mapping(line, index);
    if (!entry) continue;

    entries.push({ ...entry, sequenceDepth: sequenceIndentations.length });
    if (isBlockScalar(entry.value)) scalar = { indentation: entry.indentation };
  }

  return entries;
}

function directMappings(lines, parentIndentation) {
  const entries = structuralMappings(lines)
    .filter((entry) => entry.sequenceDepth === 0 && entry.indentation > parentIndentation);
  if (!entries.length) return [];

  const directIndentation = Math.min(...entries.map((entry) => entry.indentation));
  return entries.filter((entry) => entry.indentation === directIndentation);
}

function block(lines, parentIndentation, name) {
  const matches = directMappings(lines, parentIndentation).filter((entry) => entry.name === name);
  if (matches.length !== 1 || matches[0].value !== '') return undefined;

  const header = matches[0];
  let end = lines.length;
  for (let index = header.index + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '' || line.trimStart().startsWith('#')) continue;
    if (indentation(line) <= header.indentation) {
      end = index;
      break;
    }
  }

  return { lines: lines.slice(header.index + 1, end), indentation: header.indentation };
}

function boolean(blockValue, name) {
  const matches = directMappings(blockValue.lines, blockValue.indentation)
    .filter((entry) => entry.name === name);
  if (matches.length !== 1) return undefined;
  if (matches[0].value === 'true') return true;
  if (matches[0].value === 'false') return false;
  return undefined;
}

function validate(source) {
  const lines = source.split(/\r?\n/);
  const reviews = block(lines, -1, 'reviews');
  if (!reviews) return ['reviews block is required'];

  const autoReview = block(reviews.lines, reviews.indentation, 'auto_review');
  const errors = [];

  if (boolean(reviews, 'request_changes_workflow') !== false) {
    errors.push('reviews.request_changes_workflow must be false');
  }
  if (boolean(reviews, 'fail_commit_status') !== true) {
    errors.push('reviews.fail_commit_status must be true');
  }
  if (!autoReview || boolean(autoReview, 'enabled') !== true) {
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
