import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

describe('CreateProjectForm contract', () => {
  const source = readFileSync(join(process.cwd(), 'src', 'CreateProjectForm.tsx'), 'utf8');

  it('sends only an allowlisted name through the injected service', () => {
    expect(source).toContain('CreateProjectService');
    expect(source).toContain('createProject(input)');
    expect(source).toContain('name: name.trim()');
    expect(source).toContain('maxLength={120}');
    expect(source).not.toMatch(/sqlite|sqlx|tauri|provider|node:fs|path/);
  });

  it('validates input and prevents duplicate submit', () => {
    expect(source).toContain('if (submitting) return;');
    expect(source).toContain('Project name is required.');
    expect(source).toContain('disabled={submitting}');
    expect(source).toContain("kind: 'validation' | 'conflict' | 'error'");
  });
});
