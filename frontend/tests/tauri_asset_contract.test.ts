import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('Tauri production asset contract', () => {
  it('uses a relative base so packaged WebView assets resolve from the local app', () => {
    const viteConfig = readFileSync(resolve(__dirname, '../vite.config.ts'), 'utf8');

    expect(viteConfig).toMatch(/base:\s*['"]\.\/['"]/);
  });
});
