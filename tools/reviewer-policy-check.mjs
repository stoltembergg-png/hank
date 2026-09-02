import { readFileSync } from 'node:fs';

const configPath = process.argv[2] ?? '.coderabbit.yaml';

function indentation(line) {
  return line.length - line.trimStart().length;
}

function isBlockScalar(value) {
  return /^(?:[|>](?:[1-9])?[-+]?|[|>][-+]?[1-9])$/.test(value);
}

function isDecoratedScalar(value) {
  return /^(?:(?:![^\s]*|&[^\s]+)\s*)+/.test(value) || /^\*[^\s]+(?:\s|$)/.test(value);
}

function isFlowCollection(value) {
  return /^[\[{]/.test(value.trim());
}

function containsEmoji(value) {
  return /\p{Extended_Pictographic}/u.test(value);
}

function stripInlineComment(value) {
  const scalarStart = value.search(/\S|$/);
  let scalarQuote = value[scalarStart] === '"' || value[scalarStart] === "'" ? value[scalarStart] : undefined;
  let escaped = false;
  for (let index = scalarQuote ? scalarStart + 1 : scalarStart; index < value.length; index += 1) {
    const character = value[index];
    if (scalarQuote === '"') {
      if (escaped) {
        escaped = false;
        continue;
      }
      if (character === '\\') {
        escaped = true;
        continue;
      }
      if (character === '"') scalarQuote = undefined;
      continue;
    }
    if (scalarQuote === "'") {
      if (character === "'") {
        if (value[index + 1] === "'") {
          index += 1;
          continue;
        }
        scalarQuote = undefined;
      }
      continue;
    }
    if (character === '#' && (index === scalarStart || /\s/.test(value[index - 1]))) return value.slice(0, index);
  }
  return value;
}

function hasUnclosedQuote(value) {
  const trimmed = value.trim();
  const quote = trimmed[0];
  if (quote !== '"' && quote !== "'") return false;

  if (quote === "'") {
    for (let index = 1; index < trimmed.length; index += 1) {
      if (trimmed[index] !== "'") continue;
      if (trimmed[index + 1] === "'") {
        index += 1;
        continue;
      }
      return false;
    }
    return true;
  }

  let escaped = false;
  for (let index = 1; index < trimmed.length; index += 1) {
    const character = trimmed[index];
    if (character === '"' && !escaped) return false;
    escaped = character === '\\' && !escaped;
  }
  return true;
}

function mapping(line, index) {
  const match = /^(\s*)([A-Za-z_][A-Za-z0-9_-]*):(?:\s*(.*))?$/.exec(line);
  if (!match) return undefined;
  const value = stripInlineComment(match[3] ?? '').trim();
  return {
    index,
    indentation: match[1].length,
    name: match[2],
    value,
    syntaxError: hasUnclosedQuote(value),
  };
}

function structuralMappings(lines, syntaxErrors = []) {
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

    if (line.trim() === '' || line.trimStart().startsWith('#')) continue;
    const entry = mapping(line, index);
    if (!entry) {
      syntaxErrors.push(index);
      continue;
    }

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

function value(blockValue, name) {
  const matches = directMappings(blockValue.lines, blockValue.indentation)
    .filter((entry) => entry.name === name);
  if (matches.length !== 1) return undefined;
  return matches[0].value;
}

function rootValue(lines, name) {
  const matches = directMappings(lines, -1).filter((entry) => entry.name === name);
  if (matches.length !== 1) return undefined;
  return matches[0].value;
}

function scalarText(rawValue) {
  return /^(['"])(.*)\1$/.exec(rawValue)?.[2] ?? rawValue;
}

function decodeDoubleQuotedScalar(rawValue) {
  const trimmed = rawValue.trim();
  if (trimmed.startsWith("'")) {
    if (!/^'(?:[^']|'')*'$/.test(trimmed)) return { error: 'malformed single-quoted scalar' };
    return { text: trimmed.slice(1, -1).replace(/''/g, "'") };
  }
  if (!trimmed.startsWith('"')) return { text: scalarText(rawValue) };
  if (!trimmed.endsWith('"')) return { error: 'unterminated scalar' };

  const text = [];
  for (let index = 1; index < trimmed.length - 1; index += 1) {
    const character = trimmed[index];
    if (character === '"') return { error: 'unexpected closing quote' };
    if (character !== '\\') {
      text.push(character);
      continue;
    }

    const escape = trimmed[++index];
    const simpleEscapes = {
      '0': '\0',
      a: '\x07',
      b: '\b',
      t: '\t',
      n: '\n',
      v: '\v',
      f: '\f',
      r: '\r',
      e: '\x1b',
      ' ': ' ',
      '"': '"',
      '/': '/',
      '\\': '\\',
      N: '\u0085',
      _: '\u00a0',
      L: '\u2028',
      P: '\u2029',
    };
    if (Object.hasOwn(simpleEscapes, escape)) {
      text.push(simpleEscapes[escape]);
      continue;
    }

    const width = escape === 'x' ? 2 : escape === 'u' ? 4 : escape === 'U' ? 8 : 0;
    if (width === 0) return { error: `unsupported escape \\${escape ?? ''}` };

    const code = trimmed.slice(index + 1, index + 1 + width);
    if (!new RegExp(`^[0-9A-Fa-f]{${width}}$`).test(code)) {
      return { error: `invalid escape \\${escape}${code}` };
    }
    index += width;
    const codePoint = Number.parseInt(code, 16);
    if (escape === 'U' && codePoint > 0x10ffff) {
      return { error: `invalid escape \\${escape}${code}` };
    }
    text.push(escape === 'U' ? String.fromCodePoint(codePoint) : String.fromCharCode(codePoint));
  }

  return { text: text.join('') };
}

function requireBoolean(blockValue, name, expected, errors) {
  if (boolean(blockValue, name) !== expected) {
    errors.push(`reviews.${name} must be ${expected}`);
  }
}

function validate(source) {
  const lines = source.split(/\r?\n/);
  const syntaxErrors = [];
  const entries = structuralMappings(lines, syntaxErrors);
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

  const tone = rootValue(lines, 'tone_instructions');
  if (tone === undefined || tone === '' || isDecoratedScalar(tone) || isBlockScalar(tone) || isFlowCollection(tone)) {
    errors.push('tone_instructions must be an inline scalar');
  } else {
    const decodedTone = decodeDoubleQuotedScalar(tone);
    if (decodedTone.error) {
      errors.push(`tone_instructions contains ${decodedTone.error}`);
    } else {
      if (decodedTone.text.length > 250) errors.push('tone_instructions must be at most 250 characters');
      if (containsEmoji(decodedTone.text)) errors.push('tone_instructions must not contain emoji');
    }
  }

  if (scalarText(value(reviews, 'profile') ?? '') !== 'chill') {
    errors.push('reviews.profile must be chill');
  }

  for (const name of [
    'review_status',
    'review_details',
    'collapse_walkthrough',
    'changed_files_summary',
    'sequence_diagrams',
    'estimate_code_review_effort',
    'assess_linked_issues',
    'related_issues',
    'related_prs',
    'suggested_labels',
    'suggested_reviewers',
    'in_progress_fortune',
    'poem',
    'enable_prompt_for_ai_agents',
  ]) {
    requireBoolean(reviews, name, name === 'collapse_walkthrough', errors);
  }

  const summary = value(reviews, 'high_level_summary_instructions');
  if (summary !== undefined) {
    if (isDecoratedScalar(summary)) {
      errors.push('reviewer configuration must use untagged, unanchored scalars');
    } else if (isBlockScalar(summary) || isFlowCollection(summary)) {
      errors.push('reviews.high_level_summary_instructions must use an inline scalar');
    } else {
      const decodedSummary = decodeDoubleQuotedScalar(summary);
      if (decodedSummary.error) {
        errors.push(`reviews.high_level_summary_instructions contains ${decodedSummary.error}`);
      } else {
        if (decodedSummary.text.length > 100) errors.push('reviews.high_level_summary_instructions must be at most 100 characters');
        if (containsEmoji(decodedSummary.text)) errors.push('reviews.high_level_summary_instructions must not contain emoji');
      }
    }
  }

  if (entries.some((entry) => entry.syntaxError)) {
    errors.push('reviewer configuration contains an unclosed quoted scalar');
  }
  if (syntaxErrors.length > 0) {
    errors.push('reviewer configuration contains unsupported YAML syntax');
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
