import { createHash } from 'node:crypto';

export const POLICY_REVISION = 'review-remediation-v1';
export const MAX_FINDING_TITLE_BYTES = 512;
export const MAX_FINDING_DETAIL_BYTES = 8 * 1024;
export const MAX_FINDING_PATH_BYTES = 1 * 1024;
export const MAX_FINDING_BRANCH_BYTES = 256;
export const MAX_PATCH_FILES = 10;
export const MAX_PATCH_LINES = 500;
export const MAX_PATCH_BYTES = 64 * 1024;
export const MAX_RESULT_FILE_BYTES = 256 * 1024;

const HEX_SHA = /^[0-9a-f]{40}$/i;
const HEX_FINGERPRINT = /^[0-9a-f]{64}$/;
const CONTROL_CHARACTERS = /[\u0000-\u001f\u007f]/;
const TEXT_CONTROL_CHARACTERS = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/;
const FORBIDDEN_PATH_PREFIX = /^(?:\.github\/(?:workflows|actions)(?:\/|$)|\.git(?:\/|$)|\.env(?:\.|$))/i;
const FORBIDDEN_PATH_COMPONENT = /(?:^|\/)(?:\.env(?:\.[^/]*)?|credentials?(?:\.[^/]*)?|secrets?(?:\.[^/]*)?|tokens?(?:\.[^/]*)?|keys?(?:\.[^/]*)?|CODEOWNERS|branch-protection|rulesets?)(?:\/|$)/i;
const SECRET_ASSIGNMENT_NAME = '(?:api[_-]?key|access[_-]?token|auth(?:orization)?|client[_-]?secret|password|passwd|private[_-]?key|secret|token)';
const PEM_BLOCK = /-----BEGIN [A-Z0-9 ]+-----[\s\S]*?-----END [A-Z0-9 ]+-----/gi;
const DOUBLE_QUOTED_SECRET_ASSIGNMENT = new RegExp(`(["']?${SECRET_ASSIGNMENT_NAME}["']?\\s*[:=]\\s*)"(?:\\\\.|[^"\\\\\\r\\n])*"`, 'gi');
const SINGLE_QUOTED_SECRET_ASSIGNMENT = new RegExp(`(["']?${SECRET_ASSIGNMENT_NAME}["']?\\s*[:=]\\s*)'(?:\\\\.|[^'\\\\\\r\\n])*'`, 'gi');
const UNREDACTED_DOUBLE_QUOTED_SECRET = new RegExp(`(?:["']?${SECRET_ASSIGNMENT_NAME}["']?\\s*[:=]\\s*)"(?!\\[REDACTED\\])(?:\\\\.|[^"\\\\\\r\\n])*"`, 'i');
const UNREDACTED_SINGLE_QUOTED_SECRET = new RegExp(`(?:["']?${SECRET_ASSIGNMENT_NAME}["']?\\s*[:=]\\s*)'(?!\\[REDACTED\\])(?:\\\\.|[^'\\\\\\r\\n])*'`, 'i');
const UNREDACTED_SECRET_ASSIGNMENT = new RegExp(`(?:^|[\\s;&])${SECRET_ASSIGNMENT_NAME}\\s*[:=]\\s*(?!\\[REDACTED\\])[^\\s,;]+`, 'i');
const PEM_MATERIAL = /-----BEGIN [A-Z0-9 ]+-----[\\s\\S]*?-----END [A-Z0-9 ]+-----/i;

function byteLength(value) {
  return Buffer.byteLength(value, 'utf8');
}

function readField(input, camelName, snakeName = camelName) {
  if (input?.[camelName] !== undefined) return input[camelName];
  return input?.[snakeName];
}

function stableValue(value) {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .filter((key) => value[key] !== undefined)
        .map((key) => [key, stableValue(value[key])]),
    );
  }
  return value;
}

function stableJson(value) {
  return JSON.stringify(stableValue(value));
}

function invalid(reason) {
  return { status: 'HUMAN_REQUIRED', reason };
}

function validText(value, { label, maxBytes, required = true }) {
  if (value === undefined || value === null) {
    return required ? { error: `${label} is required` } : { value: undefined };
  }
  if (typeof value !== 'string') return { error: `${label} must be text` };
  const redacted = redactSecrets(value).trim();
  if (required && redacted.length === 0) return { error: `${label} is empty` };
  if (byteLength(redacted) > maxBytes) return { error: `${label} exceeds its byte limit` };
  if (TEXT_CONTROL_CHARACTERS.test(redacted)) return { error: `${label} contains control characters` };
  return { value: redacted };
}

function validateRepository(repository, expectedRepository) {
  if (typeof repository !== 'string' || repository !== expectedRepository) return false;
  return /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository);
}

function validateBranch(branch) {
  if (typeof branch !== 'string' || branch.length === 0 || byteLength(branch) > MAX_FINDING_BRANCH_BYTES) return false;
  if (CONTROL_CHARACTERS.test(branch) || /[\\\s~^:?*\[]/.test(branch)) return false;
  if (branch.startsWith('/') || branch.endsWith('/') || branch.startsWith('.') || branch.endsWith('.') || branch.includes('..')) return false;
  if (branch.includes('//') || branch.includes('@{') || branch === 'HEAD') return false;
  return true;
}

function validatePath(path) {
  if (typeof path !== 'string' || path.length === 0 || byteLength(path) > MAX_FINDING_PATH_BYTES) return false;
  if (CONTROL_CHARACTERS.test(path) || path.includes('\\') || path.startsWith('/') || path.includes('//')) return false;
  const segments = path.split('/');
  if (segments.some((segment) => segment === '' || segment === '.' || segment === '..')) return false;
  if (FORBIDDEN_PATH_PREFIX.test(path) || FORBIDDEN_PATH_COMPONENT.test(path)) return false;
  return true;
}

function validateEvidenceUrl(value, repository) {
  if (value === undefined || value === null || value === '') return undefined;
  if (typeof value !== 'string') return null;
  try {
    const url = new URL(value);
    const repositoryPath = `/${repository.toLowerCase()}/`;
    if (url.protocol !== 'https:' || !['github.com', 'www.github.com'].includes(url.hostname.toLowerCase())) return null;
    if (url.username || url.password || url.port || !url.pathname.toLowerCase().startsWith(repositoryPath)) return null;
    return url.toString();
  } catch {
    return null;
  }
}

function reviewerAllowed(source, reviewer) {
  if (typeof reviewer !== 'string') return false;
  const normalized = reviewer.trim().toLowerCase();
  if (source === 'coderabbit') return ['coderabbitai[bot]', 'coderabbit[bot]', 'coderabbit'].includes(normalized);
  if (source === 'aikido') return ['aikido security', 'aikido security[bot]', 'aikido[bot]', 'aikido'].includes(normalized);
  return false;
}

export function redactSecrets(value) {
  let text = String(value ?? '');
  text = text.replace(PEM_BLOCK, '[REDACTED]');
  text = text.replace(DOUBLE_QUOTED_SECRET_ASSIGNMENT, '$1"[REDACTED]"');
  text = text.replace(SINGLE_QUOTED_SECRET_ASSIGNMENT, "$1'[REDACTED]'");
  text = text.replace(/(authorization\s*:\s*)(?:bearer|basic)\s+[^\s,;]+/gi, '$1[REDACTED]');
  text = text.replace(/\b(?:bearer|basic)\s+[A-Za-z0-9+/=_-]{12,}/gi, '[REDACTED]');
  text = text.replace(new RegExp(`(^|[\\s;&])(${SECRET_ASSIGNMENT_NAME}\\s*[:=]\\s*)[^\\s,;]+`, 'gi'), '$1$2[REDACTED]');
  text = text.replace(/\b(?:sk|rk|pk)-[A-Za-z0-9][A-Za-z0-9_-]{7,}\b/g, '[REDACTED]');
  text = text.replace(/\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{12,}\b/g, '[REDACTED]');
  text = text.replace(/\bgithub_pat_[A-Za-z0-9_]{12,}\b/g, '[REDACTED]');
  text = text.replace(/\bAKIA[0-9A-Z]{12,}\b/g, '[REDACTED]');
  text = text.replace(/\bxox[baprs]-[A-Za-z0-9-]{12,}\b/g, '[REDACTED]');
  return text;
}

export function hasUnredactedSecret(value) {
  const text = String(value ?? '');
  return PEM_MATERIAL.test(text)
    || UNREDACTED_DOUBLE_QUOTED_SECRET.test(text)
    || UNREDACTED_SINGLE_QUOTED_SECRET.test(text)
    || UNREDACTED_SECRET_ASSIGNMENT.test(text);
}

export function findingFingerprint(input) {
  const fingerprintInput = {
    source: readField(input, 'source'),
    repository: readField(input, 'repository'),
    pullRequest: readField(input, 'pullRequest', 'pull_request'),
    headSha: readField(input, 'headSha', 'head_sha'),
    sourceBranch: readField(input, 'sourceBranch', 'source_branch'),
    baseBranch: readField(input, 'baseBranch', 'base_branch'),
    reviewer: readField(input, 'reviewer'),
    title: redactSecrets(readField(input, 'title') ?? ''),
    detail: redactSecrets(readField(input, 'detail') ?? ''),
    path: readField(input, 'path'),
    line: readField(input, 'line'),
    policyRevision: readField(input, 'policyRevision', 'policy_revision') ?? POLICY_REVISION,
  };
  return createHash('sha256').update(stableJson(fingerprintInput), 'utf8').digest('hex');
}

export function findingLineage(input) {
  const lineageInput = {
    source: readField(input, 'source'),
    repository: readField(input, 'repository'),
    pullRequest: readField(input, 'pullRequest', 'pull_request'),
    sourceBranch: readField(input, 'sourceBranch', 'source_branch'),
    baseBranch: readField(input, 'baseBranch', 'base_branch'),
    reviewer: readField(input, 'reviewer'),
    title: redactSecrets(readField(input, 'title') ?? ''),
    detail: redactSecrets(readField(input, 'detail') ?? ''),
    path: readField(input, 'path'),
    line: readField(input, 'line'),
    policyRevision: readField(input, 'policyRevision', 'policy_revision') ?? POLICY_REVISION,
  };
  return createHash('sha256').update(stableJson(lineageInput), 'utf8').digest('hex');
}

export function normalizeFinding(input, expectedRepository) {
  if (!input || typeof input !== 'object' || typeof expectedRepository !== 'string') return invalid('finding input is invalid');

  const sourceValue = readField(input, 'source');
  const source = typeof sourceValue === 'string' ? sourceValue.trim().toLowerCase() : '';
  if (!['aikido', 'coderabbit'].includes(source)) return invalid('finding source is not supported');

  if (!validateRepository(readField(input, 'repository'), expectedRepository)) return invalid('finding repository is not the current repository');
  const pullRequest = readField(input, 'pullRequest', 'pull_request');
  if (!Number.isSafeInteger(pullRequest) || pullRequest <= 0) return invalid('pull request number is invalid');

  const sourceBranch = readField(input, 'sourceBranch', 'source_branch');
  if (!validateBranch(sourceBranch)) return invalid('source branch is invalid');

  const baseBranch = readField(input, 'baseBranch', 'base_branch');
  if (!validateBranch(baseBranch)) return invalid('base branch is invalid');

  const headShaValue = readField(input, 'headSha', 'head_sha');
  if (typeof headShaValue !== 'string' || !HEX_SHA.test(headShaValue)) return invalid('head SHA is invalid');
  const headSha = headShaValue.toLowerCase();

  if (!reviewerAllowed(source, readField(input, 'reviewer'))) return invalid('reviewer identity is not supported');
  const reviewer = validText(readField(input, 'reviewer'), { label: 'reviewer', maxBytes: 256 });
  const title = validText(readField(input, 'title'), { label: 'title', maxBytes: MAX_FINDING_TITLE_BYTES });
  const detail = validText(readField(input, 'detail'), { label: 'detail', maxBytes: MAX_FINDING_DETAIL_BYTES });
  if (reviewer.error || title.error || detail.error) return invalid(reviewer.error || title.error || detail.error);

  const path = readField(input, 'path');
  if (!validatePath(path)) return invalid('finding path is missing or not patchable');

  const lineValue = readField(input, 'line');
  const line = lineValue === undefined || lineValue === null || lineValue === ''
    ? undefined
    : Number.isSafeInteger(lineValue) && lineValue > 0 && lineValue <= 1_000_000 ? lineValue : null;
  if (line === null) return invalid('finding line is invalid');

  const evidenceUrl = validateEvidenceUrl(readField(input, 'evidenceUrl', 'evidence_url'), expectedRepository);
  if (evidenceUrl === null) return invalid('evidence URL is invalid');

  const finding = {
    source,
    repository: expectedRepository,
    pullRequest,
    sourceBranch,
    baseBranch,
    headSha,
    reviewer: reviewer.value,
    title: title.value,
    detail: detail.value,
    path,
    line,
    evidenceUrl,
    policyRevision: POLICY_REVISION,
  };
  finding.fingerprint = findingFingerprint(finding);
  return { status: 'READY', finding };
}

export function remediationBranchName(finding) {
  const pullRequest = readField(finding, 'pullRequest', 'pull_request');
  const headSha = readField(finding, 'headSha', 'head_sha');
  const fingerprint = readField(finding, 'fingerprint');
  if (!Number.isSafeInteger(pullRequest) || !HEX_SHA.test(String(headSha ?? '')) || !HEX_FINGERPRINT.test(String(fingerprint ?? ''))) {
    throw new TypeError('cannot build remediation branch from invalid finding identity');
  }
  return `review-remediation/pr-${pullRequest}/${String(headSha).slice(0, 12).toLowerCase()}-${fingerprint.slice(0, 12)}`;
}

export function isDuplicateMarker(text, fingerprint) {
  if (!HEX_FINGERPRINT.test(String(fingerprint ?? ''))) return false;
  const marker = new RegExp(`<!--\\s*hank-review-remediation:\\s*fingerprint=${fingerprint}\\s*-->`, 'i');
  return marker.test(String(text ?? ''));
}
