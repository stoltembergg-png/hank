export type ProviderAccountState = 'connected' | 'pending' | 'revoked' | 'unavailable' | 'error';

export interface ProviderAccountStatus {
  provider_id: string;
  account_id: string;
  display_name: string;
  state: ProviderAccountState;
  has_credential_ref: boolean;
  updated_at: string;
}

export interface OAuthStartInput {
  project_id: string;
  provider_id: string;
  account_id: string;
}

export interface OAuthStartResult {
  flow_id: string;
  state: 'pending';
}

export interface OAuthFlowStatus {
  flow_id: string;
  state: 'pending' | 'connected' | 'invalid' | 'expired' | 'cancelled' | 'error';
  error_code?: 'state_mismatch' | 'redirect_mismatch' | 'provider_mismatch' | 'account_mismatch' | 'stale';
  account?: ProviderAccountStatus & { project_id?: string };
}

export interface DisconnectProviderInput {
  project_id: string;
  provider_id: string;
  account_id: string;
}

export interface ProviderSettingsApiClient {
  list(projectId: string): Promise<ProviderAccountStatus[]>;
  startOAuth(input: OAuthStartInput): Promise<OAuthStartResult>;
  getOAuthStatus(input: { project_id: string; flow_id: string }): Promise<OAuthFlowStatus>;
  disconnect(input: DisconnectProviderInput): Promise<ProviderAccountStatus>;
}

type BridgeInvoker = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface BridgeWindow {
  __TAURI_INTERNALS__?: { invoke?: BridgeInvoker };
  __TAURI_INVOKE__?: BridgeInvoker;
}

function bridgeInvoker(): BridgeInvoker | undefined {
  if (typeof window === 'undefined') return undefined;
  const bridge = window as unknown as BridgeWindow;
  return bridge.__TAURI_INTERNALS__?.invoke ?? bridge.__TAURI_INVOKE__;
}

export class DesktopProviderSettingsApiClient implements ProviderSettingsApiClient {
  async list(projectId: string): Promise<ProviderAccountStatus[]> {
    const invoke = bridgeInvoker();
    if (!invoke) return [];
    return invoke<ProviderAccountStatus[]>('list_provider_accounts', { projectId });
  }

  async startOAuth(input: OAuthStartInput): Promise<OAuthStartResult> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new Error('Provider settings service unavailable');
    return invoke<OAuthStartResult>('start_provider_oauth', { input });
  }

  async getOAuthStatus(input: { project_id: string; flow_id: string }): Promise<OAuthFlowStatus> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new Error('Provider settings service unavailable');
    return invoke<OAuthFlowStatus>('get_provider_oauth_status', { input });
  }

  async disconnect(input: DisconnectProviderInput): Promise<ProviderAccountStatus> {
    const invoke = bridgeInvoker();
    if (!invoke) throw new Error('Provider settings service unavailable');
    return invoke<ProviderAccountStatus>('disconnect_provider_account', { input });
  }
}

export const defaultProviderSettingsApi = new DesktopProviderSettingsApiClient();
