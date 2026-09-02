import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

import {
  MAX_FINDING_DETAIL_BYTES,
  MAX_FINDING_TITLE_BYTES,
  MAX_PATCH_BYTES,
  MAX_RESULT_FILE_BYTES,
  POLICY_REVISION,
  findingFingerprint,
  isDuplicateMarker,
  normalizeFinding,
  redactSecrets,
  remediationBranchName,
} from './contracts.mjs';
import { requestMimo, buildRemediationPrompt, DEFAULT_MIMO_ENDPOINT, MIMO_MODEL } from './mimo-client.mjs';
import { applyAndValidatePatch, validatePatchText } from './patch-guard.mjs';

export const MAX_REMEDIATION_CYCLES = 2;
export const MAX_REVIEW_COMMENTS = 100;
export const MAX_FILES_FOR_PROPOSAL = 100;

const CODE_RABBIT_LOGINS = new Set(['coderabbitai[bot]', 'coderabbit[bot]', 'coderabbit']);
const AIKIDO_CHECK_NAMES = new Set(['aikido security: check code', 'aikido security: deep review']);

export class OrchestratorError extends Error {
  constructor(code) {
    super(code);
    this.name = 'OrchestratorError';
    this.code = code;
  }
}

function error(code) {
  return new OrchestratorError(code);
}

function sha256(value) {
  return createHash('sha256').update(value, 'utf8').digest('hex');
}

function text(value, maxBytes) {
  if (typeof value !== 'string') return '';
  const redacted = redactSecrets(value).trim();
  return Buffer.byteLength(redacted, 'utf8') <= maxBytes ? redacted : redacted.slice(0, maxBytes);
}

function result(status, reason) {
  return { status, ...(reason ? { reason: text(reason, 512) } : {}) };
}

function eventName(event) {
  return event?.eventName ?? event?.event_name ?? event?.type ?? '';
}

function repositoryFromEvent(event) {
  return event?.repository?.full_name ?? event?.repository?.nameWithOwner;
}

function pullRequestNumberFromEvent(event) {
  if (eventName(event) === 'pull_request_review') return event?.pull_request?.number;
  if (eventName(event) === 'check_run') {
    const links = event?.check_run?.pull_requests;
    return Array.isArray(links) && links.length === 1 ? links[0]?.number : undefined;
  }
  return undefined;
}

function currentHeadSha(pullRequest) {
  return pullRequest?.head?.sha;
}

function currentSourceBranch(pullRequest) {
  return pullRequest?.head?.ref;
}

function isSameRepository(repo, repository) {
  return typeof repo?.full_name === 'string' && repo.full_name === repository;
}

function eventHeadSha(event) {
  if (eventName(event) === 'pull_request_review') return event?.review?.commit_id ?? event?.pull_request?.head?.sha;
  return event?.check_run?.head_sha ?? event?.check_run?.pull_requests?.[0]?.head?.sha;
}

function firstLine(value, fallback) {
  const line = typeof value === 'string' ? value.split(/\r?\n/, 1)[0].trim() : '';
  return line || fallback;
}

function checkAikido(check) {
  const name = typeof check?.name === 'string' ? check.name.trim().toLowerCase() : '';
  const slug = typeof check?.app?.slug === 'string' ? check.app.slug.trim().toLowerCase() : '';
  const appName = typeof check?.app?.name === 'string' ? check.app.name.trim().toLowerCase() : '';
  return AIKIDO_CHECK_NAMES.has(name) || slug === 'aikido' || appName === 'aikido security';
}

function concreteCodeRabbitComment(comments, reviewId) {
  const matching = comments.filter((comment) => comment?.pull_request_review_id === reviewId);
  if (matching.length !== 1) return undefined;
  return matching[0];
}

function concreteAikidoAnnotation(annotations) {
  const matching = annotations.filter((annotation) => typeof annotation?.path === 'string' && annotation.path.length > 0 && typeof annotation?.message === 'string' && annotation.message.trim().length > 0);
  return matching.length === 1 ? matching[0] : undefined;
}

function cycleFromComments(comments, pullRequest) {
  const pattern = new RegExp(`<!--\\s*hank-review-remediation:\\s*lineage=${pullRequest}\\s+cycle=(\\d+)\\s*-->`, 'gi');
  return comments.reduce((max, comment) => {
    const body = typeof comment?.body === 'string' ? comment.body : '';
    for (const match of body.matchAll(pattern)) max = Math.max(max, Number(match[1]));
    return max;
  }, 0);
}

async function currentPullRequest({ event, repository, api }) {
  if (repositoryFromEvent(event) && repositoryFromEvent(event) !== repository) return result('HUMAN_REQUIRED', 'event repository is foreign');
  if (event?.pull_request?.head?.repo && !isSameRepository(event.pull_request.head.repo, repository)) return result('HUMAN_REQUIRED', 'fork pull requests are not supported');
  if (event?.pull_request?.state && event.pull_request.state !== 'open') return result('NOOP', 'source pull request is not open');
  const number = pullRequestNumberFromEvent(event);
  if (!Number.isSafeInteger(number) || number <= 0) return result('HUMAN_REQUIRED', 'event is not bound to one pull request');
  let pullRequest;
  try {
    pullRequest = await api.getPullRequest(number);
  } catch {
    return result('HUMAN_REQUIRED', 'current pull request could not be read');
  }
  if (pullRequest?.state !== 'open') return result('NOOP', 'source pull request is not open');
  if (!isSameRepository(pullRequest.head?.repo, repository)) return result('HUMAN_REQUIRED', 'fork pull requests are not supported');
  if (typeof currentSourceBranch(pullRequest) !== 'string' || typeof currentHeadSha(pullRequest) !== 'string') return result('HUMAN_REQUIRED', 'pull request identity is incomplete');
  if (currentSourceBranch(pullRequest).startsWith('review-remediation/')) return result('NOOP', 'remediation branches do not retrigger remediation');
  if (event?.pull_request?.head?.sha && event.pull_request.head.sha !== currentHeadSha(pullRequest)) return result('HUMAN_REQUIRED', 'event pull request SHA is stale');
  const observedSha = eventHeadSha(event);
  if (observedSha && observedSha !== currentHeadSha(pullRequest)) return result('HUMAN_REQUIRED', 'event SHA is stale');
  return { status: 'READY', pullRequest };
}

async function finalizeFinding({ finding, api }) {
  const normalized = normalizeFinding(finding, finding.repository);
  if (normalized.status !== 'READY') return normalized;
  let issueComments;
  try {
    issueComments = await api.getIssueComments(normalized.finding.pullRequest);
  } catch {
    return result('HUMAN_REQUIRED', 'duplicate state could not be read');
  }
  if (!Array.isArray(issueComments) || issueComments.length > MAX_REVIEW_COMMENTS) return result('HUMAN_REQUIRED', 'comment history is not bounded');
  if (issueComments.some((comment) => isDuplicateMarker(comment?.body, normalized.finding.fingerprint))) return result('NOOP', 'finding fingerprint is already published');
  const previousCycle = cycleFromComments(issueComments, normalized.finding.pullRequest);
  if (previousCycle >= MAX_REMEDIATION_CYCLES) return result('HUMAN_REQUIRED', 'remediation cycle cap reached');
  let branch;
  try {
    branch = await api.getBranch(remediationBranchName(normalized.finding));
  } catch {
    return result('HUMAN_REQUIRED', 'remediation branch state could not be read');
  }
  if (branch) return result('NOOP', 'remediation branch already exists');
  return { status: 'READY', finding: normalized.finding, cycle: previousCycle + 1 };
}

export async function collectFinding({ event, repository, api }) {
  if (!api || typeof api.getPullRequest !== 'function') return result('HUMAN_REQUIRED', 'GitHub API adapter is missing');
  const current = await currentPullRequest({ event, repository, api });
  if (current.status !== 'READY') return current;
  const { pullRequest } = current;
  const number = pullRequest.number ?? pullRequestNumberFromEvent(event);
  let finding;

  if (eventName(event) === 'pull_request_review') {
    const review = event.review;
    const login = review?.user?.login;
    const state = typeof review?.state === 'string' ? review.state.toLowerCase() : '';
    if (event.action !== 'submitted' || !['changes_requested', 'commented'].includes(state)) return result('NOOP', 'review event is not actionable');
    if (!CODE_RABBIT_LOGINS.has(String(login ?? '').toLowerCase())) return result('NOOP', 'reviewer is not CodeRabbit');
    let comments;
    try {
      comments = await api.getReviewComments(number);
    } catch {
      return result('HUMAN_REQUIRED', 'review comments could not be read');
    }
    if (!Array.isArray(comments) || comments.length > MAX_REVIEW_COMMENTS) return result('HUMAN_REQUIRED', 'review comments are not bounded');
    const comment = concreteCodeRabbitComment(comments, review.id);
    if (!comment) return result('HUMAN_REQUIRED', 'review does not contain exactly one concrete inline finding');
    finding = {
      source: 'coderabbit',
      repository,
      pullRequest: number,
      sourceBranch: currentSourceBranch(pullRequest),
      headSha: currentHeadSha(pullRequest),
      reviewer: login,
      title: firstLine(comment.body, 'CodeRabbit finding'),
      detail: comment.body,
      path: comment.path,
      line: comment.line ?? comment.original_line,
      evidenceUrl: comment.html_url ?? review.html_url,
    };
  } else if (eventName(event) === 'check_run') {
    const check = event.check_run;
    if (event.action !== 'completed' || check?.conclusion === 'success') return result('NOOP', 'check event is not actionable');
    if (!checkAikido(check)) return result('NOOP', 'check is not Aikido');
    if (eventHeadSha(event) && eventHeadSha(event) !== currentHeadSha(pullRequest)) return result('HUMAN_REQUIRED', 'check SHA is stale');
    let annotations;
    try {
      annotations = await api.getCheckAnnotations(check.id);
    } catch {
      return result('HUMAN_REQUIRED', 'check annotations could not be read');
    }
    if (!Array.isArray(annotations) || annotations.length > MAX_REVIEW_COMMENTS) return result('HUMAN_REQUIRED', 'check annotations are not bounded');
    const annotation = concreteAikidoAnnotation(annotations);
    if (!annotation) return result('HUMAN_REQUIRED', 'check does not contain exactly one concrete finding');
    finding = {
      source: 'aikido',
      repository,
      pullRequest: number,
      sourceBranch: currentSourceBranch(pullRequest),
      headSha: currentHeadSha(pullRequest),
      reviewer: 'Aikido Security',
      title: firstLine(annotation.title ?? annotation.message, 'Aikido finding'),
      detail: annotation.message,
      path: annotation.path,
      line: annotation.start_line ?? annotation.end_line ?? annotation.line,
      evidenceUrl: annotation.blob_href ?? check.details_url,
    };
  } else {
    return result('NOOP', 'event type is not supported');
  }

  return finalizeFinding({ finding, api });
}

export function buildProposalInput({ finding, files }) {
  if (!finding || !Array.isArray(files) || files.length > MAX_FILES_FOR_PROPOSAL) throw error('PROPOSAL_INPUT_INVALID');
  const matches = files.filter((file) => file?.filename === finding.path);
  if (matches.length !== 1 || typeof matches[0].patch !== 'string' || matches[0].patch.length === 0) throw error('PROPOSAL_SOURCE_DIFF_MISSING');
  const patch = redactSecrets(matches[0].patch);
  if (Buffer.byteLength(patch, 'utf8') > MAX_PATCH_BYTES) throw error('PROPOSAL_SOURCE_DIFF_TOO_LARGE');
  return {
    path: finding.path,
    sourceSha: finding.headSha,
    patch,
  };
}

export function buildEvidenceDescriptor({ finding, patch, tests = [], tree = {} }) {
  if (!finding || typeof patch !== 'string') throw error('EVIDENCE_INPUT_INVALID');
  const safeTests = Array.isArray(tests) ? tests.slice(0, 20).map((command) => text(command, 512)) : [];
  const safeFiles = Array.isArray(tree?.files) ? tree.files.slice(0, 10).map((path) => text(path, 1024)) : [];
  const treeDigest = text(tree?.treeDigest ?? tree?.digest ?? '', 128);
  return {
    status: 'VALIDATED',
    repository: text(finding.repository, 256),
    pullRequest: finding.pullRequest,
    sourceBranch: text(finding.sourceBranch, 256),
    sourceSha: text(finding.headSha, 64),
    fingerprint: text(finding.fingerprint ?? findingFingerprint(finding), 64),
    policyRevision: POLICY_REVISION,
    patchDigest: sha256(patch),
    files: safeFiles,
    tests: safeTests,
    treeDigest,
  };
}

export async function proposeFinding({ collected, api, apiKey, endpoint = DEFAULT_MIMO_ENDPOINT, model = MIMO_MODEL, fetchImpl }) {
  if (collected?.status !== 'READY') return collected;
  try {
    const files = await api.getPullRequestFiles(collected.finding.pullRequest);
    const proposalInput = buildProposalInput({ finding: collected.finding, files });
    const prompt = buildRemediationPrompt({ finding: collected.finding, sourceDiff: proposalInput.patch });
    const response = await requestMimo({ apiKey, endpoint, model, prompt, fetchImpl });
    const patchMetadata = validatePatchText(response.patch);
    return {
      status: 'PROPOSED',
      viability: 'VIABLE_PATCH',
      finding: collected.finding,
      cycle: collected.cycle,
      patch: response.patch,
      patchDigest: patchMetadata.digest,
      promptDigest: prompt.digest,
      responseDigest: response.responseDigest,
    };
  } catch (caught) {
    return result('HUMAN_REQUIRED', caught?.code ?? 'proposal failed');
  }
}

export async function validateProposal({ proposed, patchFile, workspace, tests = [] }) {
  if (proposed?.status !== 'PROPOSED') return proposed;
  try {
    const applied = await applyAndValidatePatch({ workspace, patchFile });
    if (applied.digest !== proposed.patchDigest) return result('HUMAN_REQUIRED', 'patch digest changed before validation');
    const evidence = buildEvidenceDescriptor({
      finding: proposed.finding,
      patch: readPatchForEvidence(patchFile),
      tests,
      tree: applied,
    });
    return { ...evidence, finding: proposed.finding, cycle: proposed.cycle, viability: proposed.viability };
  } catch (caught) {
    return result('HUMAN_REQUIRED', caught?.code ?? 'patch validation failed');
  }
}

function readPatchForEvidence(patchFile) {
  try {
    const patch = readFileSync(patchFile, 'utf8');
    if (Buffer.byteLength(patch, 'utf8') > MAX_RESULT_FILE_BYTES) throw error('EVIDENCE_PATCH_TOO_LARGE');
    return patch;
  } catch (caught) {
    if (caught instanceof OrchestratorError) throw caught;
    throw error('EVIDENCE_PATCH_UNREADABLE');
  }
}

export function buildPublishDescriptor({ validated, finding, cycle = 1 }) {
  if (validated?.status !== 'VALIDATED' || !finding) throw error('PUBLISH_INPUT_INVALID');
  const fingerprint = finding.fingerprint ?? findingFingerprint(finding);
  const normalizedFinding = { ...finding, fingerprint };
  return {
    status: 'PUBLISH_READY',
    repository: normalizedFinding.repository,
    pullRequest: normalizedFinding.pullRequest,
    sourceBranch: normalizedFinding.sourceBranch,
    sourceSha: normalizedFinding.headSha,
    fingerprint,
    policyRevision: POLICY_REVISION,
    branch: remediationBranchName(normalizedFinding),
    base: normalizedFinding.sourceBranch,
    cycle,
    marker: `<!-- hank-review-remediation: fingerprint=${fingerprint} -->`,
    lineageMarker: `<!-- hank-review-remediation: lineage=${normalizedFinding.pullRequest} cycle=${cycle} -->`,
    patchDigest: validated.patchDigest,
    treeDigest: validated.treeDigest,
    files: validated.files,
    tests: validated.tests,
    noApproval: true,
    noMerge: true,
  };
}

export function renderDraftBody(publish) {
  if (!publish || publish.status !== 'PUBLISH_READY') throw error('PUBLISH_BODY_INPUT_INVALID');
  const files = Array.isArray(publish.files) ? publish.files.slice(0, 10).map((path) => text(path, 1024)).join(', ') : 'not recorded';
  const tests = Array.isArray(publish.tests) ? publish.tests.slice(0, 20).map((command) => `- ${text(command, 512)}`).join('\n') : '- not recorded';
  return [
    publish.marker,
    publish.lineageMarker,
    '',
    '## Reviewer remediation draft',
    '',
    `Source pull request: #${publish.pullRequest}`,
    `Source SHA: ${text(publish.sourceSha, 64)}`,
    `Finding fingerprint: ${text(publish.fingerprint, 64)}`,
    `Viability: validated bounded patch (${text(publish.patchDigest, 64)})`,
    `Files: ${files}`,
    '',
    'Checks recorded by the validation job:',
    tests,
    '',
    'This is a draft for human review. The agent did not approve, merge, rebase, or resolve the source reviewer conversation.',
    'Rollback: close this draft and delete only its automation branch; the source pull request remains unchanged.',
  ].join('\n');
}

export async function publishValidated({ validated, finding, patchFile, workspace, cycle = 1 }) {
  if (validated?.status !== 'VALIDATED') return validated;
  try {
    const reapplied = await applyAndValidatePatch({ workspace, patchFile });
    if (reapplied.digest !== validated.patchDigest || reapplied.treeDigest !== validated.treeDigest) return result('HUMAN_REQUIRED', 'publish revalidation identity changed');
    return buildPublishDescriptor({ validated, finding, cycle });
  } catch (caught) {
    return result('HUMAN_REQUIRED', caught?.code ?? 'publish validation failed');
  }
}
