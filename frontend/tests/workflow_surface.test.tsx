import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WorkflowSurface } from '@/components/WorkflowSurface';
import { DEFAULT_WORKFLOW_ID } from '@/components/WorkflowSurface';

describe('Workflow surface', () => {
  it('keeps the draft project-scoped and honest when persistence is unavailable', () => {
    render(<WorkflowSurface projectId="project-a" />);

    expect(screen.getByRole('region', { name: 'Workflows do projeto' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Workflow studio' })).toBeInTheDocument();
    expect(screen.getByText('Rascunho local')).toBeInTheDocument();
    expect(screen.getByText('A persistência de workflows ainda não está disponível no desktop.')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Adicionar nó Agent' }));

    expect(screen.getByRole('listitem', { name: 'Agent 1' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Salvar workflow' })).toBeDisabled();
  });

  it('loads and saves a versioned snapshot through the desktop workflow API', async () => {
    const api = {
      load: vi.fn().mockResolvedValue({
        project_id: 'project-a',
        workflow_id: DEFAULT_WORKFLOW_ID,
        version: 3,
        nodes: [{ id: 'agent-1', kind: 'agent', label: 'Agent 1' }],
        edges: [],
      }),
      validate: vi.fn().mockResolvedValue({ valid: true }),
      save: vi.fn().mockResolvedValue({ version: 4 }),
    };
    render(<WorkflowSurface projectId="project-a" api={api} />);

    expect(await screen.findByRole('listitem', { name: 'Agent 1' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar nó Condition' }));
    fireEvent.click(screen.getByRole('button', { name: 'Salvar workflow' }));

    await waitFor(() => expect(api.validate).toHaveBeenCalledWith(expect.objectContaining({
      project_id: 'project-a',
      workflow_id: DEFAULT_WORKFLOW_ID,
      expected_version: 3,
    })));
    await waitFor(() => expect(api.save).toHaveBeenCalled());
    expect(await screen.findByText('Workflow salvo na versão 4.')).toBeInTheDocument();
  });
});
