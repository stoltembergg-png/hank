import './UsageSummary.css';

export type UsageReadModel = {
  input_tokens: number | null;
  output_tokens: number | null;
  cost_micros: number | null;
  currency: string | null;
  currency_mismatch: boolean;
  sample_count: number;
  missing_usage_count: number;
  source: 'provider_reported' | 'estimated' | 'missing' | 'mixed';
  confidence: 'exact' | 'estimated' | 'unavailable' | 'mixed';
};

const SOURCE_LABELS: Record<UsageReadModel['source'], string> = {
  provider_reported: 'Provider informado',
  estimated: 'Estimado',
  missing: 'Uso não fornecido',
  mixed: 'Fontes mistas',
};

const CONFIDENCE_LABELS: Record<UsageReadModel['confidence'], string> = {
  exact: 'Exato',
  estimated: 'Estimado',
  unavailable: 'Indisponível',
  mixed: 'Confiança mista',
};

export function UsageSummary({ usage }: { usage: UsageReadModel }) {
  const input = usage.input_tokens === null ? 'Não fornecido' : String(usage.input_tokens);
  const output = usage.output_tokens === null ? 'Não fornecido' : String(usage.output_tokens);
  let cost = 'Custo não fornecido';
  if (usage.currency_mismatch) cost = 'Custo indisponível: moedas divergentes';
  else if (usage.cost_micros !== null && safeCurrency(usage.currency)) cost = `${usage.cost_micros} µ${usage.currency}`;

  return (
    <section className="usage-summary" aria-label="Uso de tokens" role="region">
      <h2>Uso</h2>
      <dl>
        <div><dt>Entrada</dt><dd>{input}</dd></div>
        <div><dt>Saída</dt><dd>{output}</dd></div>
        <div><dt>Custo</dt><dd>{cost}</dd></div>
      </dl>
      <p className="usage-summary-meta">
        {SOURCE_LABELS[usage.source]} · {CONFIDENCE_LABELS[usage.confidence]} · {usage.sample_count} amostra(s)
      </p>
    </section>
  );
}

function safeCurrency(value: string | null): value is string {
  return value !== null && /^[A-Z]{3,8}$/u.test(value);
}
