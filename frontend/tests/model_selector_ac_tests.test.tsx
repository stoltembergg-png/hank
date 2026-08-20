import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import {
  ModelSelectorApiClient,
  ModelSelectorOption,
  ModelSelectorSnapshot,
} from '@/api/model-selector';
import { ModelSelectorPage } from '@/providers/model-selector/ModelSelectorPage';

const snapshot: ModelSelectorSnapshot = {
  project_id: 'project_1',
  agent_id: 'agent_1',
  policy: {
    provider: 'provider-neutral',
    model: 'model-old',
    modalities: ['text'],
  },
  options: [
    {
      provider_id: 'provider-one',
      model_id: 'model-text',
      display_name: 'Provider One / Text',
      capabilities: { text: 'supported', image: 'unsupported', audio: 'unknown', video: 'unsupported' },
      state: 'available',
      source: 'provider',
    },
    {
      provider_id: 'provider-two',
      model_id: 'model-image',
      display_name: 'Provider Two / Image',
      capabilities: { text: 'supported', image: 'supported', audio: 'unsupported', video: 'unsupported' },
      state: 'available',
      source: 'cache',
    },
    {
      provider_id: 'provider-disabled',
      model_id: 'model-disabled',
      display_name: 'Provider disabled',
      capabilities: { text: 'supported', image: 'unsupported', audio: 'unsupported', video: 'unsupported' },
      state: 'disabled',
      reason: 'Provider desabilitado no registry',
      source: 'provider',
    },
    {
      provider_id: 'provider-expired',
      model_id: 'model-expired',
      display_name: 'Provider expired',
      capabilities: { text: 'supported', image: 'unsupported', audio: 'unsupported', video: 'unsupported' },
      state: 'expired',
      reason: 'Credential expirada',
      source: 'provider',
    },
    {
      provider_id: 'provider-unknown',
      model_id: 'model-unknown',
      display_name: 'Provider unknown',
      capabilities: { text: 'unknown', image: 'unknown', audio: 'unknown', video: 'unknown' },
      state: 'unavailable',
      reason: 'Capability desconhecida',
      source: 'cache',
    },
  ],
  updated_at: '2026-08-20T00:00:00.000Z',
};

function createApi(): ModelSelectorApiClient {
  return {
    get: vi.fn().mockResolvedValue(snapshot),
    update: vi.fn().mockResolvedValue({
      ...snapshot,
      policy: { ...snapshot.policy, provider: 'provider-one', model: 'model-text' },
      updated_at: '2026-08-20T00:01:00.000Z',
    }),
  };
}

describe('ModelSelectorPage', () => {
  let api: ModelSelectorApiClient;
  beforeEach(() => {
    vi.clearAllMocks();
    api = createApi();
  });

  it('renders loading state while discovery is fetched', () => {
    (api.get as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {}));
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={vi.fn()} />);
    expect(screen.getByText('Carregando modelos compatíveis...')).toBeInTheDocument();
  });

  it('renders available options and explicit reasons for disabled, expired and unknown options', async () => {
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('Provider One / Text')).toBeInTheDocument());
    expect(screen.getByText('Provider disabled')).toBeInTheDocument();
    expect(screen.getByText(/Provider desabilitado no registry/i)).toBeInTheDocument();
    expect(screen.getByText(/Credential expirada/i)).toBeInTheDocument();
    expect(screen.getByText(/Capability desconhecida/i)).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /Provider One \/ Text/i })).not.toBeDisabled();
    expect(screen.getByRole('radio', { name: /Provider disabled/i })).toBeDisabled();
  });

  it('does not silently accept unknown capability or incompatible modality', async () => {
    const imagePolicy: ModelSelectorSnapshot = {
      ...snapshot,
      policy: { ...snapshot.policy, modalities: ['image'] },
    };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue(imagePolicy);
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('Provider Two / Image')).toBeInTheDocument());
    expect(screen.getByRole('radio', { name: /Provider Two \/ Image/i })).not.toBeDisabled();
    expect(screen.getByRole('radio', { name: /Provider One \/ Text/i })).toBeDisabled();
    expect(screen.getByRole('radio', { name: /Provider unknown/i })).toBeDisabled();
  });

  it('persists an allowed selection through the typed service boundary', async () => {
    const onSaved = vi.fn();
    const onBack = vi.fn();
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={onBack} onSaved={onSaved} />);
    await waitFor(() => expect(screen.getByText('Provider One / Text')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('radio', { name: /Provider One \/ Text/i }));
    fireEvent.click(screen.getByRole('button', { name: 'Salvar seleção de modelo' }));
    await waitFor(() => expect(api.update).toHaveBeenCalledWith({
      project_id: 'project_1',
      agent_id: 'agent_1',
      provider_id: 'provider-one',
      model_id: 'model-text',
      expected_version: '2026-08-20T00:00:00.000Z',
    }));
    expect(onSaved).toHaveBeenCalled();
    expect(onBack).toHaveBeenCalled();
  });

  it('rejects stale selection conflicts without overwriting the current choice', async () => {
    (api.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('stale version'));
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('Provider One / Text')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('radio', { name: /Provider One \/ Text/i }));
    fireEvent.click(screen.getByRole('button', { name: 'Salvar seleção de modelo' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/modificada por outro processo|recarregue/i));
    expect(screen.getByRole('radio', { name: /Provider One \/ Text/i })).toBeChecked();
  });

  it('shows no-compatible-options state without inventing a fallback', async () => {
    const noOptions: ModelSelectorSnapshot = {
      ...snapshot,
      options: snapshot.options.map((option) => ({ ...option, state: 'unavailable', reason: 'Sem capability confirmada' })),
    };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue(noOptions);
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText(/Nenhum modelo compatível disponível/i)).toBeInTheDocument());
    expect(api.update).not.toHaveBeenCalled();
    expect(screen.getByText(/não haverá fallback automático/i)).toBeInTheDocument();
  });

  it('keeps secret, token and arbitrary endpoint data out of the DOM', async () => {
    const unsafeOption: ModelSelectorOption = {
      ...snapshot.options[0],
      display_name: 'model-safe',
      reason: 'api_key=[REDACTED] token=[REDACTED] endpoint=https://provider.invalid',
    };
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ ...snapshot, options: [unsafeOption] });
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('model-safe')).toBeInTheDocument());
    expect(document.body.textContent).not.toContain('api_key');
    expect(document.body.textContent).not.toContain('token');
    expect(document.body.textContent).not.toContain('https://');
  });

  it('provides accessible radiogroup, status and keyboard-actionable controls', async () => {
    render(<ModelSelectorPage projectId="project_1" agentId="agent_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('Provider One / Text')).toBeInTheDocument());
    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.getByRole('radiogroup', { name: 'Modelos disponíveis' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Salvar seleção de modelo' })).toBeInTheDocument();
  });
});
