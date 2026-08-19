import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

describe('ProjectDetail contract', () => {
  const source = readFileSync(join(process.cwd(), 'src', 'ProjectDetail.tsx'), 'utf8');

  it('uses injected services and optimistic version checks', () => {
    expect(source).toContain('ProjectDetailService');
    expect(source).toContain('UpdateProjectService');
    expect(source).toContain('ArchiveProjectService');
    expect(source).toContain('version: currentProject.version');
    expect(source).not.toMatch(/sqlite|sqlx|tauri|provider|node:fs|path/);
  });

  it('represents stale, archive confirmation and archived-state guards', () => {
    expect(source).toContain('changed elsewhere');
    expect(source).toContain('role="alertdialog"');
    expect(source).toContain('Confirm archive');
    expect(source).toContain("project.status === 'archived'");
  });
});
