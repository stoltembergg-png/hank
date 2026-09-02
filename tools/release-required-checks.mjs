#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';

const RELEASE_CHECKS = Object.freeze([
  'Build Frontend',
  'Build Rust',
  'Build Rust Windows',
  'Build Tauri Desktop',
  'Desktop E2E / Project Lifecycle',
  'w0-contract-gate',
  'CodeQL (rust)',
  'CodeQL (javascript-typescript)',
  'ONP SDD verify and audit',
  'Quality integrity',
  'Security advisory gate',
]);

const PULL_REQUEST_CHECKS = Object.freeze([
  'CodeRabbit',
  'Aikido Security: check code',
  'Aikido Security: Deep Review',
]);

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i].startsWith('--')) args[argv[i].slice(2)] = argv[++i];
  }
  return args;
}

export function readRequiredChecks(manifest) {
  if (manifest?.schemaVersion !== 1 || manifest?.branch !== 'main' || !Array.isArray(manifest.requiredChecks)) {
    throw new Error('invalid required-checks manifest');
  }
  const names = manifest.requiredChecks;
  if (!names.length || names.some((name) => typeof name !== 'string' || !name.trim())) {
    throw new Error('required-checks manifest must contain non-empty names');
  }
  if (new Set(names).size !== names.length) throw new Error('required-checks manifest contains duplicates');
  if (names.length !== RELEASE_CHECKS.length || names.some((name) => !RELEASE_CHECKS.includes(name))) {
    throw new Error('required-checks manifest must preserve the immutable release checks');
  }
  return names;
}

export function readPullRequestChecks(manifest) {
  if (!Array.isArray(manifest?.pullRequestChecks)) {
    throw new Error('required-checks manifest must contain pullRequestChecks');
  }
  const names = manifest.pullRequestChecks;
  if (names.some((name) => typeof name !== 'string' || !name.trim())) {
    throw new Error('pullRequestChecks must contain non-empty names');
  }
  if (new Set(names).size !== names.length) throw new Error('pullRequestChecks contains duplicates');
  if (names.length !== PULL_REQUEST_CHECKS.length || names.some((name) => !PULL_REQUEST_CHECKS.includes(name))) {
    throw new Error('pullRequestChecks must contain only approved reviewer checks');
  }
  return names;
}

export function readProtectedChecks(manifest) {
  const releaseChecks = readRequiredChecks(manifest);
  const pullRequestChecks = readPullRequestChecks(manifest);
  const names = [...releaseChecks, ...pullRequestChecks];
  if (new Set(names).size !== names.length) {
    throw new Error('required-checks manifest contains overlapping release and pull-request checks');
  }
  return names;
}

export function readRulesetRequiredChecks(rules) {
  if (!Array.isArray(rules)) throw new Error('active rules response must be an array');
  const required = rules.filter((rule) => rule?.type === 'required_status_checks');
  if (required.length !== 1) throw new Error(`expected exactly one active required_status_checks rule, got ${required.length}`);
  const parameters = required[0].parameters;
  if (parameters?.strict_required_status_checks_policy !== true) throw new Error('ruleset required checks must use strict policy');
  const checks = parameters.required_status_checks;
  if (!Array.isArray(checks) || !checks.length) throw new Error('ruleset has no required checks');
  const names = checks.map((check) => check?.context).filter(Boolean);
  if (new Set(names).size !== names.length) throw new Error('ruleset required checks contain duplicates');
  return names;
}

export function assertManifestMatchesRuleset({ manifestNames, rulesetNames }) {
  const ruleset = new Set(rulesetNames);
  const missingFromRuleset = manifestNames.filter((name) => !ruleset.has(name));
  if (missingFromRuleset.length) {
    throw new Error(`required check manifest/ruleset mismatch: ${JSON.stringify({ missingFromRuleset })}`);
  }
  return true;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = parseArgs(process.argv.slice(2));
  const manifest = JSON.parse(readFileSync(args.manifest, 'utf8'));
  const manifestNames = readProtectedChecks(manifest);
  const releaseNames = readRequiredChecks(manifest);
  const rulesetNames = readRulesetRequiredChecks(JSON.parse(readFileSync(args.rules, 'utf8')));
  assertManifestMatchesRuleset({ manifestNames, rulesetNames });
  writeFileSync(args.output, `${releaseNames.join(',')}\n`);
  console.log(`required checks covered by ruleset: ${manifestNames.length}; release checks emitted: ${releaseNames.length}`);
}
