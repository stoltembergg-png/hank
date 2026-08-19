import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const FRONTEND_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const REPOSITORY_ROOT = join(FRONTEND_ROOT, '..');

function productSourceFiles(): string[] {
  const sourceRoot = join(FRONTEND_ROOT, 'src');
  const files: string[] = [];

  const visit = (directory: string): void => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const fullPath = join(directory, entry.name);
      if (entry.isDirectory()) visit(fullPath);
      else if (/\.(ts|tsx)$/.test(entry.name)) files.push(fullPath);
    }
  };

  visit(sourceRoot);
  return files;
}

function readProductSource(): string {
  return productSourceFiles()
    .map((file) => readFileSync(file, 'utf8'))
    .join('\n');
}

describe('Frontend Workspace AC Tests', () => {
  // @spec:AC-201
  it('AC-201: scripts e lockfile do workspace existem', () => {
    const packageJsonPath = join(FRONTEND_ROOT, 'package.json');
    const lockfilePath = join(FRONTEND_ROOT, 'package-lock.json');
    expect(existsSync(packageJsonPath)).toBe(true);
    expect(existsSync(lockfilePath)).toBe(true);

    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
      scripts: Record<string, string>;
      dependencies: Record<string, string>;
      devDependencies: Record<string, string>;
    };
    expect(packageJson.scripts).toMatchObject({
      build: expect.any(String),
      lint: expect.any(String),
      typecheck: expect.any(String),
    });
    expect(packageJson.dependencies).toHaveProperty('react');
    expect(packageJson.devDependencies).toHaveProperty('@typescript-eslint/parser');
  });

  // @spec:AC-202
  it('AC-202: TypeScript strict sem any explícito', () => {
    const tsconfig = JSON.parse(
      readFileSync(join(FRONTEND_ROOT, 'tsconfig.json'), 'utf8'),
    ) as { compilerOptions: Record<string, unknown> };
    expect(tsconfig.compilerOptions).toMatchObject({
      strict: true,
      noUnusedLocals: true,
      noUnusedParameters: true,
    });
    expect(readProductSource()).not.toMatch(/\bany\b/);
  });

  // @spec:AC-203
  it('AC-203: imports proibidos não aparecem no produto', () => {
    const imports = [...readProductSource().matchAll(/(?:from\s+|import\s*\(\s*)['"]([^'"]+)['"]/g)].map(
      (match) => match[1],
    );
    const forbidden = /(?:sqlite|sqlx|tauri|openai|anthropic|^(?:node:)?(?:fs|path)$)/i;
    expect(imports.filter((specifier) => forbidden.test(specifier))).toEqual([]);
  });

  // @spec:AC-204
  it('AC-204: CSP Tauri v2 é restritiva e sem allowlist legada', () => {
    const tauriConfPath = join(
      REPOSITORY_ROOT,
      'apps',
      'desktop',
      'src-tauri',
      'tauri.conf.json',
    );
    const manifest = JSON.parse(readFileSync(tauriConfPath, 'utf8')) as {
      app: { security: { csp: string } };
      tauri?: unknown;
    };
    const csp = manifest.app.security.csp;
    expect(csp).toContain("default-src 'self'");
    expect(csp).toContain("script-src 'self'");
    expect(csp).not.toMatch(/unsafe-(?:inline|eval)/);
    expect(manifest.tauri).toBeUndefined();
    expect(JSON.stringify(manifest)).not.toContain('allowlist');
  });

  // @spec:AC-205
  it('AC-205: eventos de ciclo têm versão e não expõem conteúdo', () => {
    const appContent = readFileSync(join(FRONTEND_ROOT, 'src', 'App.tsx'), 'utf8');
    for (const event of ['mount', 'ready', 'unmount', 'error']) {
      expect(appContent).toContain(`event: '${event}'`);
    }
    expect(appContent).toContain('version');
    expect(appContent).toContain('timestamp');
    expect(appContent).not.toMatch(/(?:token|password|authorization|https?:\/\/)/i);
  });
});
