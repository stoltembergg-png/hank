import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import {
  ProviderIndicator,
  type ProviderIndicatorData,
} from '@/chat/indicators/ProviderIndicator';

function data(overrides: Partial<ProviderIndicatorData> = {}): ProviderIndicatorData {
  return {
    provider_id: 'provider-one',
    model_id: 'model-text',
    state: 'selected',
    capability: 'confirmed',
    attempt_number: 1,
    ...overrides,
  };
}

describe('ProviderIndicator', () => {
  it.each([
    ['selected', 'Modelo selecionado'],
    ['fallback', 'Fallback ativo'],
    ['unknown', 'Modelo desconhecido'],
    ['unavailable', 'Provider indisponível'],
    ['degraded', 'Capability degradada'],
  ] as const)('renders honest %s state', (state, label) => {
    render(<ProviderIndicator data={data({ state })} />);
    expect(screen.getByRole('status', { name: 'Status do provider' })).toHaveTextContent(label);
  });

  it('renders normalized identity and attempt without raw account/endpoint/token fields', () => {
    render(<ProviderIndicator data={data({ state: 'fallback', attempt_number: 2 })} />);
    expect(screen.getByText('provider-one')).toBeInTheDocument();
    expect(screen.getByText('model-text')).toBeInTheDocument();
    expect(screen.getByText(/tentativa 2/i)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/api_key|token|endpoint|https?:\/\//i);
  });

  it('does not claim confirmed capability for unknown or malformed metadata', () => {
    render(
      <ProviderIndicator
        data={data({
          provider_id: 'https://provider.invalid?token=secret',
          model_id: 'model safe',
          capability: 'unknown',
          state: 'unknown',
        })}
      />,
    );
    expect(screen.getByRole('status', { name: 'Status do provider' })).toHaveTextContent(/desconhecido/i);
    expect(screen.getByText('Provider não identificado')).toBeInTheDocument();
    expect(screen.getByText('Modelo não identificado')).toBeInTheDocument();
    expect(document.body.textContent).not.toContain('https://');
    expect(document.body.textContent).not.toContain('token=');
    expect(document.body.textContent).not.toContain('confirmada');
  });

  it('is accessible and stable when optional indicator metadata is absent', () => {
    render(<ProviderIndicator data={data({ provider_id: undefined, model_id: undefined, attempt_number: undefined })} />);
    expect(screen.getByRole('status', { name: 'Status do provider' })).toBeInTheDocument();
    expect(screen.getByText('Provider não identificado')).toBeInTheDocument();
    expect(screen.getByText('Modelo não identificado')).toBeInTheDocument();
  });
});
