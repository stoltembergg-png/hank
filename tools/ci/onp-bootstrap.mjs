import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

export function onpRootFrom(baseUrl) {
  return fileURLToPath(new URL('../onp-spec/', baseUrl));
}

export function portableTextSha256(content) {
  const source = Buffer.from(content);
  const normalized = Buffer.allocUnsafe(source.length);
  let writeIndex = 0;

  for (let readIndex = 0; readIndex < source.length; readIndex += 1) {
    if (source[readIndex] === 0x0d && source[readIndex + 1] === 0x0a) {
      normalized[writeIndex] = 0x0a;
      writeIndex += 1;
      readIndex += 1;
    } else {
      normalized[writeIndex] = source[readIndex];
      writeIndex += 1;
    }
  }

  return createHash('sha256').update(normalized.subarray(0, writeIndex)).digest('hex');
}
