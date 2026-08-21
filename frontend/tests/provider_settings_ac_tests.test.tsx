import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import {
  OAuthFlowStatus,
  ProviderAccountStatus,
  ProviderSettingsApiClient,
  ProviderSettingsPage,
} from '@/providers/settings/ProviderSettingsPage';

const connected: ProviderAccountStatus = {
  provider_id: 'openai',
  account_id: 'account_1',
  display_name: 'OpenAI principal',
  state: 'connected',
  has_credential_ref: true,
  updated_at: '2026-08-20T00:00:00.000Z',
};

const revoked: ProviderAccountStatus = {
  provider_id: 'anthropic',
  account_id: 'account_2',
  display_name: 'Anthropic revogado',
  state: 'revoked',
  has_credential_ref: false,
  updated_at: '2026-08-19T00:00:00.000Z',
};

function createApi(): ProviderSettingsApiClient {
  return {
    list: vi.fn().mockResolvedValue([connected, revoked]),
    startOAuth: vi.fn().mockResolvedValue({ flow_id: 'flow_1', state: 'pending' }),
    getOAuthStatus: vi.fn().mockResolvedValue({ flow_id: 'flow_1', state: 'connected', account: connected }),
    disconnect: vi.fn().mockResolvedValue({ ...connected, state: 'revoked', has_credential_ref: false }),
  };
}

describe('ProviderSettingsPage', () => {
  let api: ProviderSettingsApiClient;
  beforeEach(() => {
    vi.clearAllMocks();
    api = createApi();
  });

  it('renders loading and empty states through the service boundary', () => {
    (api.list as ReturnType<typeof vi.fn>).mockImplementation(() => new Promise(() => {}));
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    expect(screen.getByText('Carregando providers...')).toBeInTheDocument();
  });

  it('renders bounded provider status without secret or authorization code values', async () => {
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('OpenAI principal')).toBeInTheDocument());
    expect(screen.getByText('connected')).toBeInTheDocument();
    expect(screen.getByText('Credential ref opaco disponível')).toBeInTheDocument();
    expect(screen.getByText('Anthropic revogado')).toBeInTheDocument();
    expect(screen.getByText('revoked')).toBeInTheDocument();
    expect(document.body.textContent).not.toContain('api_key');
    expect(document.body.textContent).not.toContain('authorization_code');
    expect(document.body.textContent).not.toContain('secret');
    expect(document.body.textContent).not.toContain('token');
  });

  it('starts OAuth through typed service intent and shows pending state', async () => {
    (api.getOAuthStatus as ReturnType<typeof vi.fn>).mockResolvedValue({ flow_id: 'flow_1', state: 'pending' });
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('OpenAI principal')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Conectar OpenAI principal' }));
    await waitFor(() => expect(api.startOAuth).toHaveBeenCalledWith({
      project_id: 'project_1',
      provider_id: 'openai',
      account_id: 'account_1',
    }));
    expect(await screen.findByText(/OAuth pendente/i)).toBeInTheDocument();
  });

  it('shows successful OAuth callback status without rendering callback data', async () => {
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('OpenAI principal')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Conectar OpenAI principal' }));
    await waitFor(() => expect(api.getOAuthStatus).toHaveBeenCalledWith({
      project_id: 'project_1',
      flow_id: 'flow_1',
    }));
    expect(screen.getByText(/conectado com sucesso/i)).toBeInTheDocument();
    expect(document.body.textContent).not.toContain('flow_1');
  });

  it('shows invalid callback state as an explicit error and keeps account status unchanged', async () => {
    const callback: OAuthFlowStatus = { flow_id: 'flow_1', state: 'invalid', error_code: 'state_mismatch' };
    (api.getOAuthStatus as ReturnType<typeof vi.fn>).mockResolvedValue(callback);
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('OpenAI principal')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Conectar OpenAI principal' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/callback inválido|state/i));
    expect(screen.getByText('connected')).toBeInTheDocument();
    expect(api.disconnect).not.toHaveBeenCalled();
  });

  it('rejects stale callback flow from another project', async () => {
    (api.getOAuthStatus as ReturnType<typeof vi.fn>).mockResolvedValue({
      flow_id: 'flow_2',
      state: 'connected',
      account: { ...connected, project_id: 'project_2' },
    });
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('OpenAI principal')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Conectar OpenAI principal' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/desatualizado|projeto/i));
  });

  it('disconnects through typed service intent and updates status', async () => {
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('OpenAI principal')).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: 'Desconectar OpenAI principal' }));
    await waitFor(() => expect(api.disconnect).toHaveBeenCalledWith({
      project_id: 'project_1',
      provider_id: 'openai',
      account_id: 'account_1',
    }));
    expect(screen.getByRole('status')).toHaveTextContent(/revogado com sucesso/i);
  });

  it('has accessible status region and keyboard-actionable controls', async () => {
    render(<ProviderSettingsPage projectId="project_1" apiClient={api} onBack={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('OpenAI principal')).toBeInTheDocument());
    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Status dos providers' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Conectar Anthropic revogado' })).toBeInTheDocument();
  });
});
