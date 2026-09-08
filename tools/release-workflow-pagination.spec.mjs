import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = new URL('../', import.meta.url);

test('prerelease preflight slurps paginated compare responses before jq parsing', () => {
  const workflow = readFileSync(fileURLToPath(new URL('.github/workflows/release-prerelease.yml', root)), 'utf8');
  assert.match(workflow, /gh api --paginate "repos\/\$REPOSITORY\/compare\/\$PREVIOUS_STABLE_TAG\.\.\.\$SHA"/);
  assert.match(workflow, /jq -rs ['"]\.\[\].commits\[\]\?\.sha/);
  assert.doesNotMatch(workflow, /gh api --paginate "repos\/\$REPOSITORY\/compare\/\$PREVIOUS_STABLE_TAG\.\.\.\$SHA" --jq/);
});
