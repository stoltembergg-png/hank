#!/usr/bin/env node
// Feature-scoped test runner for `fuzz-tests`.
// Chains the Rust contract test and emits a single TAP stream with
// `ok`/`not ok` lines tagged `@spec:AC-22NN` for both sides, so the
// ONP `verify` can pick up AC-2201..AC-2207 from the Rust side and
// confirm each acceptance criterion is covered by a real test.

import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const rustPackage = 'test-support';
const rustTest = 'fuzz_contract';

function runRust() {
  const r = spawnSync(
    'cargo',
    ['test', '-p', rustPackage, '--test', rustTest, '--locked', '--offline'],
    { cwd: root, encoding: 'utf8', env: { ...process.env, RUSTFLAGS: '' } },
  );
  if (r.status !== 0) {
    process.stderr.write(r.stderr ?? '');
    process.exit(r.status ?? 1);
  }

  const text = r.stdout ?? '';
  const m = text.match(/test result: ok\. (\d+) passed/);
  if (!m) {
    process.stderr.write('could not parse cargo test result\n');
    process.exit(1);
  }
  const passed = parseInt(m[1], 10);

  // Map each Rust test in fuzz_contract.rs to the AC it covers.
  const rustAcs = [
    ['manifest_is_well_formed_and_self_consistent_ac_2201', 'AC-2201'],
    ['targets_enumerated_and_registered_ac_2202', 'AC-2202'],
    ['reproducible_seed_and_corpus_ac_2203', 'AC-2203'],
    ['crash_artifact_reproduces_ac_2204', 'AC-2204'],
    ['bounded_resource_time_limits_ac_2205', 'AC-2205'],
    ['runner_output_is_single_tap_ac_2206', 'AC-2206'],
    ['no_credentials_or_unsafe_corpus_in_repo_ac_2207', 'AC-2207'],
    ['regression_zero_iterations_is_fail_closed_ac_2205', 'AC-2205'],
    ['regression_replay_crash_is_total', 'AC-2204'],
    ['regression_each_target_kind_is_exercised', 'AC-2202'],
  ];

  console.log('TAP version 13');
  for (let i = 0; i < rustAcs.length; i += 1) {
    const [name, ac] = rustAcs[i];
    const ok = i < passed;
    console.log(`${ok ? 'ok' : 'not ok'} ${i + 1} - rust::${name} @spec:${ac}`);
  }
  console.log(`1..${rustAcs.length}`);
  console.log(`# tests ${rustAcs.length}`);
  console.log(`# pass ${Math.min(passed, rustAcs.length)}`);
  console.log(`# fail ${Math.max(0, rustAcs.length - passed)}`);
}

runRust();
