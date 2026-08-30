import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ProductShell } from '@/components/ProductShell';

describe('Product shell', () => {
  it('renders the real workspace entry and keeps unavailable modules honest', () => {
    render(
      <ProductShell enabledSections={['overview']}>
        <p>Conteúdo do workspace</p>
      </ProductShell>,
    );

    expect(screen.getByRole('complementary', { name: 'Navegação principal' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Visão geral' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByText('Conteúdo do workspace')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Conversas' }))
      .toHaveAttribute('title', 'Disponível ao integrar o fluxo');
    expect(screen.getByRole('button', { name: 'Conversas' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Workflows' })).toBeDisabled();
    expect(screen.getAllByText('Em integração').length).toBeGreaterThan(0);
  });

  it('exposes the active workspace context in the shell header', () => {
    render(
      <ProductShell enabledSections={['overview']}>
        <p>Conteúdo do workspace</p>
      </ProductShell>,
    );

    expect(screen.getByRole('banner', { name: 'Cabeçalho do workspace' })).toBeInTheDocument();
    expect(screen.getByText('Hank Desktop', { exact: true })).toBeVisible();
    expect(screen.getByText('Workspace local', { exact: true })).toBeVisible();
  });

  it('notifies a supported navigation selection without changing the content contract', () => {
    const onSectionChange = vi.fn();

    render(
      <ProductShell enabledSections={['overview']} onSectionChange={onSectionChange}>
        <p>Conteúdo do workspace</p>
      </ProductShell>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Visão geral' }));

    expect(onSectionChange).toHaveBeenCalledWith('overview');
  });
});
