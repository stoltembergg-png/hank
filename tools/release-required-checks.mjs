#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';

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
  const manifest = new Set(manifestNames);
  const ruleset = new Set(rulesetNames);
  const missingFromRuleset = manifestNames.filter((name) => !ruleset.has(name));
  const missingFromManifest = rulesetNames.filter((name) => !manifest.has(name));
  if (missingFromRuleset.length || missingFromManifest.length || manifest.size !== ruleset.size) {
    throw new Error(`required check manifest/ruleset mismatch: ${JSON.stringify({ missingFromRuleset, missingFromManifest })}`);
  }
  return true;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = parseArgs(process.argv.slice(2));
  const manifestNames = readRequiredChecks(JSON.parse(readFileSync(args.manifest, 'utf8')));
  const rulesetNames = readRulesetRequiredChecks(JSON.parse(readFileSync(args.rules, 'utf8')));
  assertManifestMatchesRuleset({ manifestNames, rulesetNames });
  writeFileSync(args.output, `${rulesetNames.join(',')}\n`);
  console.log(`required checks manifest/ruleset match: ${rulesetNames.length}`);
}
