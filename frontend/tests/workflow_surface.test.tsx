import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { WorkflowSurface } from '@/components/WorkflowSurface';

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
});
