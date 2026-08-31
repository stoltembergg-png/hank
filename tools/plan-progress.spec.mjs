import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import test from 'node:test';

import {
  calculateProgress,
  extractPlannedCards,
  parseMergedPullRequests,
  renderProgressSection,
  replaceProgressSection,
} from './plan-progress.mjs';

test('extracts ordered unique plan cards from queue headings', () => {
  const queue = [
    '# Queue',
    '### PR-002 — second',
    '### PR-001 — first',
    '### PR-002 — duplicate must fail closed',
  ].join('\n');

  assert.throws(() => extractPlannedCards([queue]), /duplicate plan card PR-002/);
  assert.deepEqual(extractPlannedCards(['### PR-002 — second\n### PR-001 — first']), [1, 2]);
});

test('current executable queue is complete and starts at the first card', () => {
  const queueTexts = readdirSync(new URL('../.planning/queue/', import.meta.url))
    .filter((file) => /^queue-.*\.md$/u.test(file))
    .sort()
    .map((file) => readFileSync(new URL(`../.planning/queue/${file}`, import.meta.url), 'utf8'));
  const cards = extractPlannedCards(queueTexts);

  assert.equal(cards[0], 1);
  assert.equal(cards.at(-1), 414);
  assert.equal(cards.length, 414);
});

test('counts merged work PRs while excluding the progress automation branch', () => {
  const planned = [389, 390, 391];
  const merged = parseMergedPullRequests([
    { number: 389, merged_at: '2026-08-31T07:52:25Z', head: { ref: 'codex/workflow-surface' } },
    { number: 390, merged_at: '2026-08-31T08:00:00Z', head: { ref: 'automation/plan-progress' } },
    {
      number: 392,
      merged_at: '2026-08-31T08:05:00Z',
      head: { ref: 'feat/next-card' },
      body: 'Plan card: PR-390',
    },
    {
      number: 393,
      merged_at: '2026-08-31T08:10:00Z',
      head: { ref: 'docs/plan-progress' },
      body: 'Plan card: none',
    },
  ]);

  assert.deepEqual(calculateProgress(planned, merged), {
    completed: 2,
    total: 3,
    percentage: 67,
    latestMerged: { number: 393, mergedAt: '2026-08-31T08:10:00Z' },
    nextCard: 391,
    gaps: [391],
    unmappedWork: 1,
  });
});

test('renders and replaces exactly one README progress section', () => {
  const progress = calculateProgress([1, 2, 3], [
    { number: 1, mergedAt: '2026-08-30T00:00:00Z', headRef: 'feat/one' },
  ]);
  const section = renderProgressSection(progress);

  assert.match(section, /HANK_PLAN_PROGRESS:START/);
  assert.match(section, /1\/3 IDs do plano/);
  assert.match(section, /PR-002/);
  assert.match(section, /IDs do plano sem PR correspondente/);
  assert.equal(replaceProgressSection(`before\n${section}\nafter`, section), `before\n${section}\nafter`);
  assert.throws(() => replaceProgressSection('without markers', section), /exactly one/);
  assert.throws(() => replaceProgressSection(`${section}\n${section}`, section), /exactly one/);
});

test('workflow updates a bot PR and does not push directly to protected main', () => {
  const workflow = readFileSync(new URL('../.github/workflows/update-plan-progress.yml', import.meta.url), 'utf8');

  assert.match(workflow, /push:\s*\n\s+branches:\s*\[main\]/);
  assert.match(workflow, /contents:\s*write/);
  assert.match(workflow, /pull-requests:\s*write/);
  assert.match(workflow, /automation\/plan-progress/);
  assert.match(workflow, /gh pr create/);
  assert.doesNotMatch(workflow, /git push[^\n]*\bmain\b/);
});
