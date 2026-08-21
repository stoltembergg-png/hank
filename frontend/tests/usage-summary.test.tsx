import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { UsageSummary, type UsageReadModel } from '@/chat/usage/UsageSummary';

const model: UsageReadModel = {
  input_tokens: 10,
  output_tokens: 5,
  cost_micros: 42,
  currency: 'USD',
  currency_mismatch: false,
  sample_count: 2,
  missing_usage_count: 0,
  source: 'provider_reported',
  confidence: 'exact',
};

describe('UsageSummary', () => {
  it('renders provider-reported tokens/cost and explicit source/confidence', () => {
    render(<UsageSummary usage={model} />);
    expect(screen.getByRole('region', { name: 'Uso de tokens' })).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();
    expect(screen.getByText(/42 µUSD/i)).toBeInTheDocument();
    expect(screen.getByText(/Provider informado/i)).toBeInTheDocument();
    expect(screen.getByText(/Exato/i)).toBeInTheDocument();
  });

  it('keeps missing usage optional instead of displaying fake zero values', () => {
    render(
      <UsageSummary
        usage={{ ...model, input_tokens: null, output_tokens: null, cost_micros: null, currency: null, source: 'missing', confidence: 'unavailable', missing_usage_count: 1 }}
      />,
    );
    expect(screen.getByText(/Uso não fornecido/i)).toBeInTheDocument();
    expect(screen.queryByText('0')).not.toBeInTheDocument();
    expect(screen.getByText(/Indisponível/i)).toBeInTheDocument();
  });

  it('does not claim a cost when currencies mismatch and renders estimated state honestly', () => {
    render(<UsageSummary usage={{ ...model, cost_micros: null, currency: null, currency_mismatch: true, source: 'mixed', confidence: 'mixed' }} />);
    expect(screen.getByText(/Custo indisponível: moedas divergentes/i)).toBeInTheDocument();
    expect(screen.getByText(/Fontes mistas/i)).toBeInTheDocument();
    expect(screen.getByText(/Confiança mista/i)).toBeInTheDocument();
  });
});
