import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ModelPolicyPage } from '@/agents/builder/model/ModelPolicyPage';
import {
  ModelModality,
  ModelPolicyApiClient,
  ModelPolicySnapshot,
} from '@/api/agent-model-policy';

const snapshot: ModelPolicySnapshot = {
  policy: {
    provider: 'provider-neutral',
    model: 'model-abstract',
    max_tokens: 4096,
    max_context_tokens: 16_384,
    temperature: 0.2,
    modalities: ['text'],
  },
  capabilities: {
    text: 'supported',
    image: 'unknown',
    audio: 'unsupported',
    video: 'unsupported',
  },
  provider_state: 'available',
  updated_at: '2026-01-01T00:00:00.000Z',
};

function createApiClient(): ModelPolicyApiClient {
  return {
    get: vi.fn().mockResolvedValue(snapshot),
    update: vi.fn().mockResolvedValue(snapshot),
  };
}

describe('ModelPolicyPage', () => {
  let apiClient: ModelPolicyApiClient;
  const onBack = vi.fn();
  const onSaved = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    apiClient = createApiClient();
  });

  it('renders loading state while policy is fetched', () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {}));

    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    expect(screen.getByText('Carregando política de modelo...')).toBeInTheDocument();
  });

  it('renders provider-neutral fields and capability states', async () => {
    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    expect(screen.getByDisplayValue('model-abstract')).toBeInTheDocument();
    expect(screen.getByDisplayValue('4096')).toBeInTheDocument();
    expect(screen.getByDisplayValue('16384')).toBeInTheDocument();
    expect(screen.getByDisplayValue('0.2')).toBeInTheDocument();
    expect(screen.getByText('text: supported')).toBeInTheDocument();
    expect(screen.getByText('image: unknown')).toBeInTheDocument();
    expect(screen.getByText('audio: unsupported')).toBeInTheDocument();
    expect(screen.getByText(/provider-neutral, sem SDK concreto/i)).toBeInTheDocument();
  });

  it('updates only provider-neutral allowlisted policy fields', async () => {
    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
        onSaved={onSaved}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Provider ID abstrato'), {
      target: { value: 'provider-two' },
    });
    fireEvent.change(screen.getByLabelText('Model ID abstrato'), {
      target: { value: 'model-two' },
    });
    fireEvent.change(screen.getByLabelText('Max tokens'), { target: { value: '8192' } });
    fireEvent.change(screen.getByLabelText('Janela de contexto (tokens)'), {
      target: { value: '32768' },
    });
    fireEvent.change(screen.getByLabelText('Temperatura'), { target: { value: '0.7' } });
    fireEvent.click(screen.getByLabelText('Modalidade image'));

    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de modelo' }));

    await waitFor(() => {
      expect(apiClient.update).toHaveBeenCalledWith({
        project_id: 'prj_1',
        agent_id: 'agt_1',
        policy: {
          provider: 'provider-two',
          model: 'model-two',
          max_tokens: 8192,
          max_context_tokens: 32768,
          temperature: 0.7,
          modalities: ['text', 'image'],
        },
        expected_version: '2026-01-01T00:00:00.000Z',
      });
    });

    const payload = (apiClient.update as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(payload.policy).not.toHaveProperty('parameters');
    expect(payload.policy).not.toHaveProperty('endpoint');
    expect(payload.policy).not.toHaveProperty('api_key');
    expect(onSaved).toHaveBeenCalledWith(snapshot);
    expect(onBack).toHaveBeenCalled();
  });

  it('rejects endpoint-like provider identifiers', async () => {
    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Provider ID abstrato'), {
      target: { value: 'https://provider.invalid' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de modelo' }));

    expect(screen.getByRole('alert')).toHaveTextContent(/URL|endpoint|identificador inválido/i);
    expect(apiClient.update).not.toHaveBeenCalled();
  });

  it('rejects invalid numeric limits and temperature', async () => {
    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Max tokens'), { target: { value: '0' } });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de modelo' }));
    expect(screen.getByRole('alert')).toHaveTextContent(/Max tokens deve estar entre/i);
    expect(apiClient.update).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText('Max tokens'), { target: { value: '4096' } });
    fireEvent.change(screen.getByLabelText('Temperatura'), { target: { value: '2.1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de modelo' }));
    expect(screen.getByRole('alert')).toHaveTextContent(/Temperatura deve estar entre/i);
    expect(apiClient.update).not.toHaveBeenCalled();
  });

  it('requires at least one modality and rejects unsupported duplicate state', async () => {
    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByLabelText('Modalidade text'));
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de modelo' }));

    expect(screen.getByRole('alert')).toHaveTextContent(/modalidade/i);
    expect(apiClient.update).not.toHaveBeenCalled();
  });

  it('shows explicit unsupported/no-provider state without inventing support', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue(null);

    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText(/Nenhum provider disponível/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/suporte de modelo é desconhecido ou indisponível/i)).toBeInTheDocument();
    expect(screen.queryByLabelText(/API key|endpoint|token/i)).not.toBeInTheDocument();
  });

  it('shows a provider-available snapshot with unsupported capabilities explicitly', async () => {
    (apiClient.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...snapshot,
      provider_state: 'unsupported',
    });

    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    expect(screen.getByText(/Nenhum provider compatível foi negociado/i)).toBeInTheDocument();
  });

  it('maps stale updates to a conflict and preserves the form', async () => {
    (apiClient.update as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('stale version'));

    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText('Model ID abstrato'), {
      target: { value: 'model-new' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Salvar política de modelo' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(/modificada por outro processo/i);
    });
    expect(screen.getByDisplayValue('model-new')).toBeInTheDocument();
    expect(onBack).not.toHaveBeenCalled();
  });

  it('supports cancel without changes and confirms unsaved changes', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);

    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: 'Salvar política de modelo' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));
    expect(onBack).toHaveBeenCalledTimes(1);

    fireEvent.change(screen.getByLabelText('Model ID abstrato'), {
      target: { value: 'model-new' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Cancelar' }));
    expect(confirmSpy).toHaveBeenCalled();
    expect(onBack).toHaveBeenCalledTimes(1);
    confirmSpy.mockRestore();
  });

  it('exposes accessible labels and never renders credential controls', async () => {
    render(
      <ModelPolicyPage
        projectId="prj_1"
        agentId="agt_1"
        apiClient={apiClient}
        onBack={onBack}
      />,
    );

    await waitFor(() => {
      expect(screen.getByDisplayValue('provider-neutral')).toBeInTheDocument();
    });

    expect(screen.getByRole('heading', { name: 'Política de modelo do Agent' })).toBeInTheDocument();
    expect(screen.getByRole('form', { name: 'Formulário de política de modelo' })).toBeInTheDocument();
    expect(screen.getByLabelText('Modalidade text')).toBeInTheDocument();
    expect(screen.queryByLabelText(/API key|endpoint|password/i)).not.toBeInTheDocument();
  });
});