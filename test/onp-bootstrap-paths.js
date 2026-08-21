import assert from 'node:assert/strict';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { onpRootFrom, portableTextSha256 } from '../tools/ci/onp-bootstrap.mjs';

test('AC-016: ONP bootstrap resolves its embedded root as a native path @spec:AC-016', () => {
  const bootstrapUrl = new URL('../tools/ci/run-onp-spec.mjs', import.meta.url);
  const expected = fileURLToPath(new URL('../onp-spec/', bootstrapUrl));

  assert.equal(onpRootFrom(bootstrapUrl), expected);
});

test('AC-016: ONP snapshot hashes are stable across CRLF checkouts @spec:AC-016', () => {
  assert.equal(
    portableTextSha256(Buffer.from('alpha\r\nbeta\r\n')),
    'e49c81e2d2f84e259d40e2fb8192f3bcd198b355184845d76d8f58807d0d78ee',
  );
});

test('AC-016: ONP snapshot hashing preserves non-text bytes @spec:AC-016', () => {
  assert.notEqual(portableTextSha256(Buffer.from([0xff])), portableTextSha256(Buffer.from([0xfe])));
});
