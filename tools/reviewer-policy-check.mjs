import { readFileSync } from 'node:fs';

const configPath = process.argv[2] ?? '.coderabbit.yaml';

function indentation(line) {
  return line.length - line.trimStart().length;
}

function block(lines, name) {
  const header = new RegExp(`^\\s*${name}:\\s*(?:#.*)?$`);
  const start = lines.findIndex((line) => header.test(line));
  if (start === -1) return undefined;

  const headerIndentation = indentation(lines[start]);
  let end = lines.length;
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() === '' || line.trimStart().startsWith('#')) continue;
    if (indentation(line) <= headerIndentation) {
      end = index;
      break;
    }
  }

  return lines.slice(start + 1, end);
}

function boolean(lines, name) {
  const matcher = new RegExp(`^\\s*${name}:\\s*(true|false)\\s*(?:#.*)?$`);
  const line = lines.find((candidate) => matcher.test(candidate));
  if (!line) return undefined;
  return matcher.exec(line)[1] === 'true';
}

function validate(source) {
  const lines = source.split(/\r?\n/);
  const reviews = block(lines, 'reviews');
  if (!reviews) return ['reviews block is required'];

  const autoReview = block(reviews, 'auto_review');
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
