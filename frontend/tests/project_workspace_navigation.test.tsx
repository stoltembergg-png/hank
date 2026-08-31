import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import App from '../src/App';
import type { ProjectSummary } from '../src/types/project';

const project: ProjectSummary = {
  id: 'prj_01j7x000000000000000000042',
  name: 'Projeto de Navegação',
  description: 'Projeto real usado para validar o workspace.',
  status: 'active',
  owner: 'gabriel',
  created_at: '2026-08-30T12:00:00.000Z',
  updated_at: '2026-08-30T12:00:00.000Z',
};

describe('Project workspace navigation', () => {
  it('makes the selected project Agents workspace reachable from the sidebar', async () => {
    const invoke = vi.fn(async (command: string) => {
      switch (command) {
        case 'frontend_ready':
          return { stage: 'APPLICATION_READY' };
        case 'list_projects':
          return { projects: [project], total: 1, limit: 10, offset: 0 };
        case 'list_agents':
          return { agents: [], total: 0, limit: 10, offset: 0 };
        case 'list_memories':
          return { project_id: project.id, memories: [] };
        case 'list_skills':
          return {
            project_id: project.id,
            scope: 'project',
            skills: [],
            total: 0,
            limit: 50,
            offset: 0,
            available: true,
          };
        case 'list_scheduled_jobs':
          return [];
        default:
          throw new Error(`unexpected desktop command: ${command}`);
      }
    });
    const bridgeWindow = window as Window & { __TAURI_INTERNALS__?: unknown };
    const previousBridge = bridgeWindow.__TAURI_INTERNALS__;
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: { invoke },
    });

    try {
      render(<App />);
      fireEvent.click(await screen.findByRole('listitem', { name: `Ver detalhes de ${project.name}` }));

      expect(screen.getByRole('button', { name: 'Visão geral' })).toBeEnabled();
      const agentsButton = screen.getByRole('button', { name: 'Agents' });
      expect(agentsButton).toBeEnabled();
      for (const section of ['Conversas', 'Workflows', 'Skills', 'Memória', 'Configurações']) {
        expect(screen.getByRole('button', { name: section })).toBeDisabled();
      }

      fireEvent.click(agentsButton);
      expect(agentsButton).toHaveAttribute('aria-current', 'page');
      await screen.findByRole('heading', { name: /Agents/ });
      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith(
          'list_agents',
          { input: expect.objectContaining({ project_id: project.id }) },
        );
      });
    } finally {
      Object.defineProperty(window, '__TAURI_INTERNALS__', {
        configurable: true,
        value: previousBridge,
      });
    }
  });
});
