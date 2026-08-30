import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ProjectList } from '../src/components/ProjectList';
import { ProjectApiClient } from '../src/api/projects';

describe('Projects visual surface', () => {
  it('exposes project tools and metadata without changing the project API flow', async () => {
    const apiClient: ProjectApiClient = {
      list: vi.fn().mockResolvedValue({
        projects: [
          {
            id: 'prj_visual_001',
            name: 'Visual Workspace',
            description: 'Workspace used for visual validation',
            status: 'active',
            owner: 'gabriel',
            created_at: '2026-08-30T12:00:00.000Z',
            updated_at: '2026-08-30T12:00:00.000Z',
          },
        ],
        total: 1,
        limit: 10,
        offset: 0,
      }),
    };

    render(<ProjectList apiClient={apiClient} />);

    expect(screen.getByRole('region', { name: 'Gerenciamento de Projetos' })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Carregando projetos...');

    await waitFor(() => {
      expect(screen.getByRole('listitem', { name: 'Ver detalhes de Visual Workspace' })).toBeInTheDocument();
    });

    expect(screen.getByRole('toolbar', { name: 'Ferramentas de projetos' })).toBeInTheDocument();
    expect(screen.getByText('Organize seus projetos e sessões.')).toBeInTheDocument();
    expect(screen.getByText('1 projeto')).toBeInTheDocument();

    const card = screen.getByRole('listitem', { name: 'Ver detalhes de Visual Workspace' });
    expect(card).toHaveTextContent('active');
    expect(card.querySelector('.project-card-icon')).not.toBeNull();
    expect(card.querySelector('.project-card-open')).not.toBeNull();
  });
});
