import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const manifestPath = path.join(root, 'docs', 'evidence-scope-manifest.json');
const SHA = /^[0-9a-f]{40,64}$/i;

const expected = new Map([
  ['PR-261', { feature: 'fuzz-tests', pullRequest: 447, boundary: 'offline-synthetic' }],
  ['PR-266', { feature: 'release-signing', pullRequest: 452, boundary: 'offline-synthetic-key-fixtures' }],
  ['PR-267', { feature: 'installers', pullRequest: 453, boundary: 'contract-only-platform-matrix' }],
]);

function requireString(value, label) {
  if (typeof value !== 'string' || value.length === 0) throw new Error(`${label} must be a non-empty string`);
}

function requireIncludes(values, expectedValue, label) {
  if (!Array.isArray(values) || !values.some((value) => value.toLowerCase().includes(expectedValue.toLowerCase()))) {
    throw new Error(`${label} must include ${expectedValue}`);
  }
}

export function readManifest() {
  return JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
}

export function assertSourceClean({ projectRoot = root } = {}) {
  const result = spawnSync('git', ['status', '--porcelain=v1', '-z'], { cwd: projectRoot, encoding: 'buffer' });
  if (result.error || result.status !== 0) throw new Error(`git status failed: ${result.stderr?.toString() || result.error?.message || result.status}`);
  const records = result.stdout.toString('utf8').split('\\0').filter(Boolean);
  const paths = [];
  for (let index = 0; index < records.length; index += 1) {
    const record = records[index];
    if (record.length < 4) throw new Error('malformed git status record');
    const code = record.slice(0, 2);
    paths.push(record.slice(3));
    if (code[0] === 'R' || code[0] === 'C') {
      const original = records[++index];
      if (!original) throw new Error('incomplete rename/copy status record');
      paths.push(original);
    }
  }
  const unexpected = paths.filter((candidate) => candidate
    && !candidate.startsWith('.spec/verification/')
    && !candidate.startsWith('security/reports/'));
  if (unexpected.length) throw new Error(`source tree is dirty; commit before generating exact evidence: ${unexpected.join(', ')}`);
  return true;
}

export function validateManifest(manifest, { projectRoot = root } = {}) {
  if (manifest?.schemaVersion !== 1 || manifest.kind !== 'contract-evidence-scope') throw new Error('invalid evidence scope manifest');
  if (manifest.policy?.productionStatus !== 'NO_PROOF means production behavior was not exercised') throw new Error('production boundary policy is missing');
  if (!Array.isArray(manifest.entries) || manifest.entries.length !== expected.size) throw new Error('evidence scope must contain exactly three entries');

  const seen = new Set();
  for (const entry of manifest.entries) {
    requireString(entry.logicalCard, 'logicalCard');
    if (seen.has(entry.logicalCard)) throw new Error(`duplicate logicalCard ${entry.logicalCard}`);
    seen.add(entry.logicalCard);
    const rule = expected.get(entry.logicalCard);
    if (!rule) throw new Error(`unexpected logicalCard ${entry.logicalCard}`);
    if (entry.feature !== rule.feature || entry.mergedPullRequest !== rule.pullRequest || entry.contractBoundary !== rule.boundary) {
      throw new Error(`${entry.logicalCard} identity or boundary diverges from the approved scope`);
    }
    if (entry.contractStatus !== 'PASS' || entry.productionStatus !== 'NO_PROOF' || entry.visualStatus !== 'contract-result-card-only') {
      throw new Error(`${entry.logicalCard} must remain PASS/NO_PROOF/contract-result-card-only`);
    }
    requireString(entry.testCommand, `${entry.logicalCard}.testCommand`);
    if (!entry.testCommand.startsWith('node ')) throw new Error(`${entry.logicalCard}.testCommand must be a bounded Node command`);
    if (!Array.isArray(entry.provenClaims) || entry.provenClaims.length < 2) throw new Error(`${entry.logicalCard}.provenClaims is incomplete`);
    if (!Array.isArray(entry.notProven) || entry.notProven.length < 2) throw new Error(`${entry.logicalCard}.notProven is incomplete`);
    if (!Array.isArray(entry.sources) || entry.sources.length < 4) throw new Error(`${entry.logicalCard}.sources is incomplete`);
    for (const source of entry.sources) {
      if (typeof source !== 'string' || source.startsWith('/') || source.includes('..')) throw new Error(`${entry.logicalCard} has an unsafe source path`);
      if (!fs.existsSync(path.join(projectRoot, source))) throw new Error(`${entry.logicalCard} source is missing: ${source}`);
    }
  }
  for (const card of expected.keys()) if (!seen.has(card)) throw new Error(`missing logicalCard ${card}`);
  requireIncludes(manifest.entries.find((entry) => entry.logicalCard === 'PR-261').notProven, 'production parser', 'PR-261 notProven');
  requireIncludes(manifest.entries.find((entry) => entry.logicalCard === 'PR-266').notProven, 'real release artifact', 'PR-266 notProven');
  requireIncludes(manifest.entries.find((entry) => entry.logicalCard === 'PR-267').notProven, 'real installer', 'PR-267 notProven');
  return true;
}

export function buildReport({ manifest = readManifest(), sourceCommit, sourceTree, generatedAt, testSummary }) {
  if (!SHA.test(sourceCommit ?? '') || !SHA.test(sourceTree ?? '')) throw new Error('report identity must contain full Git commit and tree SHA');
  validateManifest(manifest);
  if (!testSummary || testSummary.plan !== manifest.entries.length || testSummary.passed !== manifest.entries.length || testSummary.failed !== 0 || testSummary.skipped !== 0) {
    throw new Error('report requires one passing result for every scope entry');
  }
  return {
    schemaVersion: 1,
    kind: 'contract-evidence-report',
    generatedAt,
    sourceCommit,
    sourceTree,
    boundary: 'contract-only; production integration is NO_PROOF',
    testSummary,
    entries: manifest.entries.map((entry) => ({
      logicalCard: entry.logicalCard,
      mergedPullRequest: entry.mergedPullRequest,
      feature: entry.feature,
      title: entry.title,
      contractStatus: entry.contractStatus,
      productionStatus: entry.productionStatus,
      visualStatus: entry.visualStatus,
      provenClaims: entry.provenClaims,
      notProven: entry.notProven,
    })),
  };
}

function escapeXml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' })[character]);
}

function wrap(text, width = 90) {
  const words = String(text).split(/\s+/);
  const lines = [];
  let line = '';
  for (const word of words) {
    if (line && `${line} ${word}`.length > width) { lines.push(line); line = word; } else line = line ? `${line} ${word}` : word;
  }
  if (line) lines.push(line);
  return lines;
}

export function renderSvg(report) {
  const cardHeight = 206;
  const width = 1500;
  const height = 158 + report.entries.length * cardHeight;
  const out = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title desc">`,
    '<title id="title">Hank contract evidence scope</title>',
    '<desc id="desc">Real contract test results with explicit NO_PROOF production boundaries.</desc>',
    '<rect width="100%" height="100%" fill="#0b1220"/>',
    '<rect x="32" y="26" width="1436" height="92" rx="14" fill="#172554" stroke="#60a5fa" stroke-width="2"/>',
    '<text x="60" y="64" fill="#dbeafe" font-family="sans-serif" font-size="28" font-weight="700">HANK CONTRACT EVIDENCE</text>',
    '<text x="60" y="96" fill="#fbbf24" font-family="sans-serif" font-size="22" font-weight="700">NOT PRODUCTION PROOF — contract execution only</text>',
    `<text x="1040" y="60" fill="#bfdbfe" font-family="monospace" font-size="16">commit ${escapeXml(report.sourceCommit)}</text>`,
    `<text x="1040" y="84" fill="#bfdbfe" font-family="monospace" font-size="16">tree   ${escapeXml(report.sourceTree)}</text>`,
    `<text x="1040" y="108" fill="#bfdbfe" font-family="sans-serif" font-size="15">${escapeXml(report.testSummary.passed)}/${escapeXml(report.testSummary.plan)} scope checks PASS</text>`,
  ];
  report.entries.forEach((entry, index) => {
    const y = 138 + index * cardHeight;
    out.push(`<rect x="32" y="${y}" width="1436" height="184" rx="14" fill="#111827" stroke="#334155" stroke-width="2"/>`);
    out.push(`<text x="60" y="${y + 34}" fill="#f8fafc" font-family="sans-serif" font-size="24" font-weight="700">${escapeXml(entry.logicalCard)} — ${escapeXml(entry.title)}</text>`);
    out.push(`<text x="60" y="${y + 64}" fill="#86efac" font-family="monospace" font-size="18" font-weight="700">CONTRACT: ${escapeXml(entry.contractStatus)}</text>`);
    out.push(`<text x="350" y="${y + 64}" fill="#fbbf24" font-family="monospace" font-size="18" font-weight="700">PRODUCTION: ${escapeXml(entry.productionStatus)}</text>`);
    out.push(`<text x="60" y="${y + 94}" fill="#93c5fd" font-family="sans-serif" font-size="16" font-weight="700">Proven by this contract:</text>`);
    wrap(entry.provenClaims[0], 100).slice(0, 2).forEach((line, lineIndex) => out.push(`<text x="80" y="${y + 118 + lineIndex * 20}" fill="#cbd5e1" font-family="sans-serif" font-size="15">• ${escapeXml(line)}</text>`));
    out.push(`<text x="770" y="${y + 94}" fill="#fca5a5" font-family="sans-serif" font-size="16" font-weight="700">Explicitly not proven:</text>`);
    wrap(entry.notProven[0], 72).slice(0, 3).forEach((line, lineIndex) => out.push(`<text x="790" y="${y + 118 + lineIndex * 20}" fill="#cbd5e1" font-family="sans-serif" font-size="15">• ${escapeXml(line)}</text>`));
    out.push(`<text x="60" y="${y + 166}" fill="#94a3b8" font-family="monospace" font-size="13">visual=${escapeXml(entry.visualStatus)} · merged PR #${escapeXml(entry.mergedPullRequest)}</text>`);
  });
  out.push('</svg>');
  return out.join('\n');
}
