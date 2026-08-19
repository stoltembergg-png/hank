import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

describe('ProjectList contract', () => {
  const source = readFileSync(join(process.cwd(), 'src', 'ProjectList.tsx'), 'utf8');

  it('uses an injected application service and bounded pagination', () => {
    expect(source).toContain('ListProjectsService');
    expect(source).toContain('listProjects(page, pageSize)');
    expect(source).toContain('pageSize = 20');
    expect(source).not.toMatch(/sqlite|sqlx|tauri|provider/i);
  });

  it('represents loading, empty and error states', () => {
    expect(source).toContain('Loading projects');
    expect(source).toContain('No projects yet');
    expect(source).toContain('Unable to load projects');
    expect(source).toContain('role="alert"');
  });
});
