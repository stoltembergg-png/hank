import './ProviderIndicator.css';

export type ProviderIndicatorState = 'selected' | 'fallback' | 'unknown' | 'unavailable' | 'degraded';
export type ProviderCapabilityState = 'confirmed' | 'unknown' | 'unsupported';

export type ProviderIndicatorData = {
  provider_id?: string;
  model_id?: string;
  state: ProviderIndicatorState;
  capability: ProviderCapabilityState;
  attempt_number?: number;
};

const STATE_LABELS: Record<ProviderIndicatorState, string> = {
  selected: 'Modelo selecionado',
  fallback: 'Fallback ativo',
  unknown: 'Modelo desconhecido',
  unavailable: 'Provider indisponível',
  degraded: 'Capability degradada',
};

const CAPABILITY_LABELS: Record<ProviderCapabilityState, string> = {
  confirmed: 'Capability confirmada',
  unknown: 'Capability desconhecida',
  unsupported: 'Capability não suportada',
};

export function ProviderIndicator({ data }: { data: ProviderIndicatorData }) {
  const provider = safeIdentifier(data.provider_id);
  const model = safeIdentifier(data.model_id);
  const attempt = Number.isInteger(data.attempt_number) && data.attempt_number && data.attempt_number > 0 && data.attempt_number <= 1000
    ? data.attempt_number
    : undefined;
  const stateLabel = STATE_LABELS[data.state] ?? STATE_LABELS.unknown;
  const capabilityLabel = CAPABILITY_LABELS[data.capability] ?? CAPABILITY_LABELS.unknown;

  return (
    <div className={`provider-indicator provider-indicator-${data.state}`}>
      <div className="provider-indicator-status" role="status" aria-label="Status do provider">
        {stateLabel}
      </div>
      <div className="provider-indicator-details">
        <span>{provider ?? 'Provider não identificado'}</span>
        <span>{model ?? 'Modelo não identificado'}</span>
        <span>{capabilityLabel}</span>
        {attempt && <span aria-label={`Tentativa ${attempt}`}>tentativa {attempt}</span>}
      </div>
    </div>
  );
}

function safeIdentifier(value?: string): string | undefined {
  if (!value
    || value.length > 128
    || value.includes('://')
    || value.includes('token')
    || value.includes('secret')
    || value.split('').some((character) => {
      const code = character.charCodeAt(0);
      return code <= 0x1f || code === 0x7f || /\s/u.test(character);
    })) {
    return undefined;
  }
  return value;
}
